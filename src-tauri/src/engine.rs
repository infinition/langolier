//! Embedded llama.cpp engine, on its own thread.
use crate::db::Res;
use llama_cpp_2::{
    context::params::LlamaContextParams,
    llama_backend::LlamaBackend,
    llama_batch::LlamaBatch,
    model::{params::LlamaModelParams, AddBos, LlamaChatMessage, LlamaModel},
    sampling::LlamaSampler,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::path::{Path, PathBuf};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc, Arc, OnceLock,
};
use std::time::{Duration, Instant};

/// Seconds a model may sit unused before it is unloaded; 0 keeps it forever.
static IDLE_SECS: AtomicU64 = AtomicU64::new(300);
pub fn set_idle_secs(secs: u64) {
    IDLE_SECS.store(secs, Ordering::Relaxed);
}
/// Where curated tags such as `qwen3:4b-instruct` resolve to a GGUF.
static CACHE_DIR: OnceLock<PathBuf> = OnceLock::new();
pub fn set_cache_dir(dir: PathBuf) {
    let _ = CACHE_DIR.set(dir);
}

pub struct GenParams {
    pub temperature: f32,
    pub max_tokens: usize,
    pub n_ctx: u32,
}
pub struct GenOut {
    pub text: String,
    pub tokens: u64,
}
enum Req {
    Generate {
        model: PathBuf,
        messages: Vec<(String, String)>,
        params: GenParams,
        cancel: Arc<AtomicBool>,
        on_token: tokio::sync::mpsc::UnboundedSender<String>,
        done: tokio::sync::oneshot::Sender<Res<GenOut>>,
    },
    Embed {
        model: PathBuf,
        texts: Vec<String>,
        done: tokio::sync::oneshot::Sender<Res<Vec<Vec<f32>>>>,
    },
    /// Unloads models and frees the backend before the process exits.
    Shutdown(mpsc::Sender<()>),
}
static QUEUE: OnceLock<std::sync::Mutex<mpsc::Sender<Req>>> = OnceLock::new();

fn queue() -> &'static std::sync::Mutex<mpsc::Sender<Req>> {
    QUEUE.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Req>();
        std::thread::Builder::new()
            .name("llama".into())
            .spawn(move || worker(rx))
            .expect("thread llama");
        std::sync::Mutex::new(tx)
    })
}
fn send(req: Req) -> Res<()> {
    queue()
        .lock()
        .map_err(|_| "Embedded engine unavailable")?
        .send(req)
        .map_err(|_| "Embedded engine stopped".into())
}
pub fn resolve(path: &str) -> Res<PathBuf> {
    let p = PathBuf::from(path.trim());
    if p.is_absolute() {
        return Ok(p);
    }
    // A curated tag maps to its cached GGUF.
    if let (Some(spec), Some(dir)) = (crate::llm::chat_model(path), CACHE_DIR.get()) {
        return Ok(dir.join(spec.gguf_file));
    }
    if let (Some(dir), Some(spec)) = (
        CACHE_DIR.get(),
        crate::llm::EMBED_MODELS
            .iter()
            .find(|m| m.tag == path.trim()),
    ) {
        return Ok(dir.join(spec.gguf_file));
    }
    let base = crate::bundle::bundle_dir()?;
    let candidate = base.join(&p);
    if candidate.exists() {
        return Ok(candidate);
    }
    Ok(p)
}
pub fn available(path: &str) -> bool {
    resolve(path).map(|p| p.is_file()).unwrap_or(false)
}
pub async fn generate(
    model: &str,
    messages: Vec<(String, String)>,
    params: GenParams,
    cancel: Arc<AtomicBool>,
    on_token: impl Fn(&str),
) -> Res<GenOut> {
    let model = resolve(model)?;
    if !model.is_file() {
        return Err(format!("Model not found: {}", model.display()));
    }
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    send(Req::Generate {
        model,
        messages,
        params,
        cancel,
        on_token: tx,
        done: done_tx,
    })?;
    while let Some(piece) = rx.recv().await {
        on_token(&piece);
    }
    done_rx.await.map_err(|_| "The engine did not answer")?
}
/// Call before quitting if the engine was ever started.
pub fn shutdown() {
    if let Some(q) = QUEUE.get() {
        let (tx, rx) = mpsc::channel();
        if q.lock()
            .map(|s| s.send(Req::Shutdown(tx)).is_ok())
            .unwrap_or(false)
        {
            let _ = rx.recv_timeout(std::time::Duration::from_secs(20));
        }
    }
}
pub async fn embed(model: &str, texts: &[String]) -> Res<Vec<Vec<f32>>> {
    let model = resolve(model)?;
    if !model.is_file() {
        return Err(format!("Embedding model not found: {}", model.display()));
    }
    let (done_tx, done_rx) = tokio::sync::oneshot::channel();
    send(Req::Embed {
        model,
        texts: texts.to_vec(),
        done: done_tx,
    })?;
    done_rx.await.map_err(|_| "The engine did not answer")?
}

struct Worker {
    backend: LlamaBackend,
    models: HashMap<PathBuf, LlamaModel>,
}
impl Worker {
    /// Loads the model on demand, then serves it from the cache.
    fn ensure(&mut self, path: &Path) -> Res<()> {
        if !self.models.contains_key(path) {
            eprintln!("Loading model {}…", path.display());
            let params = LlamaModelParams::default()
                .with_n_gpu_layers(if cfg!(target_os = "macos") { 1000 } else { 0 });
            let model = LlamaModel::load_from_file(&self.backend, path, &params)
                .map_err(|e| format!("Chargement de {} : {e}", path.display()))?;
            self.models.insert(path.to_path_buf(), model);
        }
        Ok(())
    }
}
fn worker(rx: mpsc::Receiver<Req>) {
    let mut backend = match LlamaBackend::init() {
        Ok(b) => b,
        Err(e) => {
            eprintln!("llama.cpp : {e}");
            return;
        }
    };
    if std::env::var_os("LANGOLIER_LLAMA_LOGS").is_none() {
        backend.void_logs();
    }
    let mut w = Worker {
        backend,
        models: HashMap::new(),
    };
    let mut last_used = Instant::now();
    loop {
        // Wake up regularly to release memory once the models sit idle.
        let req = match rx.recv_timeout(Duration::from_secs(30)) {
            Ok(r) => r,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                let idle = IDLE_SECS.load(Ordering::Relaxed);
                if idle > 0 && !w.models.is_empty() && last_used.elapsed().as_secs() >= idle {
                    w.models.clear();
                }
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        last_used = Instant::now();
        match req {
            Req::Generate {
                model,
                messages,
                params,
                cancel,
                on_token,
                done,
            } => {
                let r = run_generate(&mut w, &model, &messages, &params, &cancel, &on_token);
                drop(on_token);
                let _ = done.send(r);
            }
            Req::Embed { model, texts, done } => {
                let r = run_embed(&mut w, &model, &texts);
                let _ = done.send(r);
            }
            Req::Shutdown(ack) => {
                w.models.clear();
                drop(w);
                let _ = ack.send(());
                return;
            }
        }
    }
}
fn run_generate(
    w: &mut Worker,
    path: &Path,
    messages: &[(String, String)],
    params: &GenParams,
    cancel: &AtomicBool,
    on_token: &tokio::sync::mpsc::UnboundedSender<String>,
) -> Res<GenOut> {
    w.ensure(path)?;
    let (model, backend) = (&w.models[path], &w.backend);
    let template = model
        .chat_template(None)
        .map_err(|e| format!("Template de conversation : {e}"))?;
    // Thinking models such as Qwen3 need /no_think.
    let thinking_template = model
        .meta_val_str("tokenizer.chat_template")
        .map(|t| t.contains("enable_thinking"))
        .unwrap_or(false);
    let mut messages = messages.to_vec();
    if thinking_template {
        if let Some(last) = messages.iter_mut().rev().find(|(role, _)| role == "user") {
            last.1.push_str(" /no_think");
        }
    }
    let chat = messages
        .iter()
        .map(|(role, content)| LlamaChatMessage::new(role.clone(), content.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    let prompt = model
        .apply_chat_template(&template, &chat, true)
        .map_err(|e| format!("Application du template : {e}"))?;
    let tokens = model
        .str_to_token(&prompt, AddBos::Always)
        .map_err(|e| e.to_string())?;
    let n_ctx = params.n_ctx.clamp(2048, model.n_ctx_train().max(2048));
    if tokens.len() as u32 + 16 > n_ctx {
        return Err(format!(
            "Context too short: {} prompt tokens for {n_ctx}.",
            tokens.len()
        ));
    }
    let batch_size = 512usize;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(n_ctx))
        .with_n_batch(batch_size as u32)
        .with_n_ubatch(batch_size as u32);
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Contexte : {e}"))?;
    let mut batch = LlamaBatch::new(batch_size, 1);
    // Prompt in slices; only the last token's logits matter.
    for (start, chunk) in tokens
        .chunks(batch_size)
        .enumerate()
        .map(|(i, c)| (i * batch_size, c))
    {
        batch.clear();
        for (j, t) in chunk.iter().enumerate() {
            let pos = start + j;
            batch
                .add(*t, pos as i32, &[0], pos + 1 == tokens.len())
                .map_err(|e| e.to_string())?;
        }
        ctx.decode(&mut batch)
            .map_err(|e| format!("Decoding the prompt: {e}"))?;
    }
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u32)
        .unwrap_or(42);
    let mut sampler = if params.temperature <= 0.01 {
        LlamaSampler::greedy()
    } else {
        LlamaSampler::chain_simple([
            LlamaSampler::min_p(0.05, 1),
            LlamaSampler::top_p(0.95, 1),
            LlamaSampler::temp(params.temperature),
            LlamaSampler::dist(seed),
        ])
    };
    let mut decoder = encoding_rs::UTF_8.new_decoder();
    let mut out = String::new();
    let mut pos = tokens.len();
    let mut produced = 0u64;
    // Thinking models open with <think>...</think>; that is filtered out.
    enum Phase {
        Start(String),
        Think(String),
        Answer,
    }
    let mut phase = Phase::Start(String::new());
    // Nothing is emitted before the first visible character.
    let emit = |s: &str, out: &mut String| {
        let s = if out.is_empty() { s.trim_start() } else { s };
        if !s.is_empty() {
            out.push_str(s);
            let _ = on_token.send(s.to_string());
        }
    };
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Generation cancelled".into());
        }
        if produced as usize >= params.max_tokens || pos as u32 + 1 >= n_ctx {
            break;
        }
        let token = sampler.sample(&ctx, batch.n_tokens() - 1);
        sampler.accept(token);
        if model.is_eog_token(token) {
            break;
        }
        produced += 1;
        let piece = model
            .token_to_piece(token, &mut decoder, false, None)
            .unwrap_or_default();
        phase = match phase {
            Phase::Start(mut buf) => {
                buf.push_str(&piece);
                let t = buf.trim_start().to_string();
                if let Some(rest) = t.strip_prefix("<think>") {
                    Phase::Think(rest.to_string())
                } else if !"<think>".starts_with(&t) || buf.len() > 16 {
                    emit(&buf, &mut out);
                    Phase::Answer
                } else {
                    Phase::Start(buf)
                }
            }
            Phase::Think(mut buf) => {
                buf.push_str(&piece);
                match buf.find("</think>") {
                    Some(i) => {
                        let rest = buf[i + "</think>".len()..].trim_start().to_string();
                        emit(&rest, &mut out);
                        Phase::Answer
                    }
                    None => Phase::Think(buf),
                }
            }
            Phase::Answer => {
                emit(&piece, &mut out);
                Phase::Answer
            }
        };
        batch.clear();
        batch
            .add(token, pos as i32, &[0], true)
            .map_err(|e| e.to_string())?;
        pos += 1;
        ctx.decode(&mut batch)
            .map_err(|e| format!("Decoding: {e}"))?;
    }
    if let Phase::Start(buf) = phase {
        emit(&buf, &mut out);
    }
    Ok(GenOut {
        text: out,
        tokens: produced,
    })
}
fn run_embed(w: &mut Worker, path: &Path, texts: &[String]) -> Res<Vec<Vec<f32>>> {
    w.ensure(path)?;
    let (model, backend) = (&w.models[path], &w.backend);
    let n_batch = 2048u32;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(NonZeroU32::new(n_batch))
        .with_n_batch(n_batch)
        .with_n_ubatch(n_batch)
        .with_embeddings(true);
    let mut ctx = model
        .new_context(backend, ctx_params)
        .map_err(|e| format!("Contexte embeddings : {e}"))?;
    let mut out = Vec::with_capacity(texts.len());
    let mut batch = LlamaBatch::new(n_batch as usize, 1);
    for text in texts {
        let mut tokens = model
            .str_to_token(text, AddBos::Always)
            .map_err(|e| e.to_string())?;
        tokens.truncate(n_batch as usize - 8);
        batch.clear();
        for (i, t) in tokens.iter().enumerate() {
            batch
                .add(*t, i as i32, &[0], true)
                .map_err(|e| e.to_string())?;
        }
        ctx.clear_kv_cache();
        ctx.decode(&mut batch)
            .map_err(|e| format!("Embeddings : {e}"))?;
        let v = ctx.embeddings_seq_ith(0).map_err(|e| e.to_string())?;
        let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        out.push(if norm > 0.0 {
            v.iter().map(|x| x / norm).collect()
        } else {
            v.to_vec()
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    /// Compares what this engine produces against the vectors already stored,
    /// to say whether relabelling a library is safe or whether it has to be
    /// indexed again.
    /// LANGOLIER_COMPARE_DB=/path/langolier.sqlite3 LANGOLIER_GGUF_EMBED=/path/model.gguf \
    ///   cargo test compare_stored -- --ignored --nocapture
    #[test]
    #[ignore = "Needs a populated library and the embedding model"]
    fn compare_stored() {
        let Ok(db) = std::env::var("LANGOLIER_COMPARE_DB") else {
            return;
        };
        let emb = std::env::var("LANGOLIER_GGUF_EMBED").expect("LANGOLIER_GGUF_EMBED");
        let key = std::env::var("LANGOLIER_COMPARE_KEY")
            .unwrap_or_else(|_| "http://127.0.0.1:11434|embeddinggemma".to_string());
        let conn = rusqlite::Connection::open_with_flags(
            &db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )
        .unwrap();
        let mut stmt = conn
            .prepare(
                "SELECT text, embedding FROM chunks \
                 WHERE embedding IS NOT NULL AND embedding_model = ?1 \
                 AND length(text) BETWEEN 200 AND 1200 \
                 ORDER BY id LIMIT 24",
            )
            .unwrap();
        let rows: Vec<(String, Vec<u8>)> = stmt
            .query_map([&key], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert!(!rows.is_empty(), "no passage carries the key {key}");
        let texts: Vec<String> = rows.iter().map(|(t, _)| t.clone()).collect();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let fresh = rt.block_on(embed(&emb, &texts)).unwrap();
        let mut scores = Vec::with_capacity(rows.len());
        for ((_, blob), got) in rows.iter().zip(&fresh) {
            let (whole, _) = blob.as_chunks::<4>();
            let stored: Vec<f32> = whole.iter().copied().map(f32::from_le_bytes).collect();
            assert_eq!(stored.len(), got.len(), "dimensions differ");
            let dot: f32 = stored.iter().zip(got).map(|(a, b)| a * b).sum();
            let ns: f32 = stored.iter().map(|x| x * x).sum::<f32>().sqrt();
            let ng: f32 = got.iter().map(|x| x * x).sum::<f32>().sqrt();
            scores.push(dot / (ns * ng));
        }
        scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean = scores.iter().sum::<f32>() / scores.len() as f32;
        println!(
            "COMPARE {} passages | min {:.4} | median {:.4} | mean {:.4} | max {:.4}",
            scores.len(),
            scores[0],
            scores[scores.len() / 2],
            mean,
            scores[scores.len() - 1]
        );
        // A different engine on the same passage lands far below this.
        println!(
            "VERDICT {}",
            if scores[0] > 0.99 {
                "relabelling is safe"
            } else {
                "index again"
            }
        );
    }
    #[test]
    #[ignore = "Charge de vrais modeles ; cargo test engine -- --ignored --nocapture"]
    fn embed_and_generate() {
        let chat = std::env::var("LANGOLIER_GGUF_CHAT").expect("LANGOLIER_GGUF_CHAT");
        let emb = std::env::var("LANGOLIER_GGUF_EMBED").expect("LANGOLIER_GGUF_EMBED");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let texts = vec![
            "Qu'est-ce qu'un transformer ?".to_string(),
            "Un transformer est une architecture de réseau de neurones fondée sur l'attention."
                .to_string(),
            "Recette de la tarte aux pommes : peler les pommes.".to_string(),
        ];
        let cos = |a: &[f32], b: &[f32]| a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>();
        if emb != "skip" {
            let t = std::time::Instant::now();
            let v = rt.block_on(embed(&emb, &texts)).unwrap();
            println!(
                "EMBED dims={} en {:.1}s | question~definition={:.3} question~tarte={:.3}",
                v[0].len(),
                t.elapsed().as_secs_f64(),
                cos(&v[0], &v[1]),
                cos(&v[0], &v[2])
            );
            for (i, vec) in v.iter().enumerate() {
                println!(
                    "VEC{i} {}",
                    vec.iter()
                        .take(6)
                        .map(|x| format!("{x:.4}"))
                        .collect::<Vec<_>>()
                        .join(",")
                );
            }
            assert!(cos(&v[0], &v[1]) > cos(&v[0], &v[2]));
        }
        let t = std::time::Instant::now();
        let out = rt
            .block_on(generate(
                &chat,
                vec![
                    (
                        "system".into(),
                        "Réponds en une phrase, en français.".into(),
                    ),
                    ("user".into(), "Qu'est-ce qu'un transformer ?".into()),
                ],
                GenParams {
                    temperature: 0.2,
                    max_tokens: 120,
                    n_ctx: 4096,
                },
                Arc::new(AtomicBool::new(false)),
                |p| print!("{p}"),
            ))
            .unwrap();
        println!(
            "\nGEN {} tokens en {:.1}s ({:.1} tok/s)",
            out.tokens,
            t.elapsed().as_secs_f64(),
            out.tokens as f64 / t.elapsed().as_secs_f64()
        );
        assert!(!out.text.is_empty());
        shutdown();
    }
}
