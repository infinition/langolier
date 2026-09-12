//! Server mode: raw HTTP/1.1 and SSE, no framework.
use crate::{
    assistant, bundle,
    db::{err, Db, Res},
    llm, pipeline,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Mutex, RwLock};

const PAGE: &str = include_str!("../web/chat.html");

pub struct Args {
    pub host: String,
    pub port: u16,
    pub open: bool,
    pub data: Option<PathBuf>,
    /// Native window rather than a browser.
    pub gui: bool,
    pub explicit_port: bool,
}
/// Mode comes from the arguments, else from the executable name.
pub fn parse_args(argv: &[String]) -> Option<Args> {
    let explicit = argv.iter().any(|a| a == "--serve" || a == "--gui");
    let stem = std::env::current_exe()
        .ok()
        .and_then(|p| p.file_stem().map(|s| s.to_string_lossy().to_lowercase()))
        .unwrap_or_default();
    let by_name = if stem.contains("chatbotgui") {
        Some(true)
    } else if stem.contains("chatbotweb") {
        Some(false)
    } else {
        None
    };
    if !explicit && by_name.is_none() {
        return None;
    }
    let mut a = Args {
        host: "127.0.0.1".into(),
        port: 8787,
        open: by_name == Some(false),
        data: None,
        gui: by_name == Some(true) || argv.iter().any(|x| x == "--gui"),
        explicit_port: false,
    };
    let mut it = argv.iter().skip(1);
    while let Some(x) = it.next() {
        match x.as_str() {
            "--host" => a.host = it.next().cloned().unwrap_or(a.host),
            "--port" => {
                a.port = it.next().and_then(|p| p.parse().ok()).unwrap_or(a.port);
                a.explicit_port = true;
            }
            "--open" => a.open = true,
            "--data" => a.data = it.next().map(PathBuf::from),
            "--bundle" => {
                if let Some(d) = it.next() {
                    std::env::set_var("LANGOLIER_BUNDLE_DIR", d);
                }
            }
            _ => {}
        }
    }
    Some(a)
}
struct App {
    db: Db,
    swap: RwLock<()>,
    /// One generation at a time, web page and Telegram bridge together.
    generation_shared: Arc<Mutex<()>>,
}
pub fn main(args: Args) -> i32 {
    if args.gui {
        #[cfg(feature = "desktop")]
        return gui(args);
        #[cfg(not(feature = "desktop"))]
        {
            eprintln!("Ce binaire est un serveur sans interface : utilisez --serve.");
            return 2;
        }
    }
    let rt = tokio::runtime::Runtime::new().expect("tokio");
    let code = match rt.block_on(serve(args)) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Erreur : {e}");
            1
        }
    };
    crate::engine::shutdown();
    code
}
#[cfg(feature = "desktop")]
fn gui(mut args: Args) -> i32 {
    args.host = "127.0.0.1".into();
    args.port = 0;
    let (tx, rx) = std::sync::mpsc::channel::<Result<(String, String), String>>();
    let tx_err = tx.clone();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio");
        let result = rt.block_on(serve_with(args, move |url, name| {
            let _ = tx.send(Ok((url, name)));
        }));
        if let Err(e) = result {
            let _ = tx_err.send(Err(e));
        }
    });
    let (url, name) = match rx.recv() {
        Ok(Ok(v)) => v,
        Ok(Err(e)) => {
            eprintln!("Erreur : {e}");
            return 1;
        }
        Err(_) => {
            eprintln!("Error: the local server did not start.");
            return 1;
        }
    };
    let title = name.clone();
    let result = tauri::Builder::default()
        .setup(move |app| {
            let target: tauri::Url = url.parse().map_err(std::io::Error::other)?;
            tauri::WebviewWindowBuilder::new(app, "chat", tauri::WebviewUrl::External(target))
                .title(&title)
                .inner_size(560.0, 800.0)
                .min_inner_size(380.0, 520.0)
                .build()?;
            Ok(())
        })
        .run(crate::context());
    crate::engine::shutdown();
    match result {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("Window: {e}");
            1
        }
    }
}
pub async fn serve(args: Args) -> Res<()> {
    serve_with(args, |_, _| {}).await
}
/// ready(url, name) fires once the port is open; window mode uses it.
pub async fn serve_with(
    args: Args,
    ready: impl FnOnce(String, String) + Send + 'static,
) -> Res<()> {
    let exe_dir = bundle::bundle_dir()?;
    let data_dir = match args.data {
        Some(d) => d,
        None => {
            let cands = bundle::candidates(&exe_dir)?;
            let slug = match cands.last() {
                Some((_, bytes)) => {
                    let tmp = std::env::temp_dir()
                        .join(format!("langolier-probe-{}.langolier", std::process::id()));
                    std::fs::write(&tmp, bytes).map_err(err)?;
                    let s = bundle::slug_of(&tmp);
                    let _ = std::fs::remove_file(&tmp);
                    s
                }
                None => "chatbot".into(),
            };
            bundle::default_data_dir(&slug)?
        }
    };
    match bundle::sync(&exe_dir, &data_dir)? {
        Some(name) => println!("Bundle installed: {name}"),
        None if data_dir.join("langolier.sqlite3").exists() => {
            println!("Bundle already installed.")
        }
        None => {
            return Err(format!(
                "No .langolier file found in {} (nor embedded in the executable).",
                exe_dir.display()
            ))
        }
    }
    let db = Db::new(&data_dir)?;
    let s = db.settings()?;
    let profile = assistant::get(&db, &s.active_assistant)?;
    println!(
        "Assistant : {}",
        profile
            .as_ref()
            .map(|a| a.name.as_str())
            .unwrap_or("(profil absent)")
    );
    println!("Moteur : {} · {}", s.provider, s.model);
    if s.provider == "ollama"
        || (s.embedding_endpoint.trim() != llm::EMBEDDED && !llm::is_cloud(&s.provider))
    {
        prepare_ollama(&db).await;
    }
    let shared = Arc::new(Mutex::new(()));
    let app = Arc::new(App {
        db,
        swap: RwLock::new(()),
        generation_shared: shared,
    });
    // Telegram bridge when the profile carries a token.
    if profile
        .as_ref()
        .is_some_and(|a| !a.telegram_token.trim().is_empty())
    {
        println!("Telegram : pont actif pour ce profil.");
    }
    {
        let db = app.db.clone();
        let lock = app.generation_shared.clone();
        tokio::spawn(crate::telegram::poll(db, lock));
    }
    // Requested port, then the next ones.
    let explicit = args.explicit_port;
    let mut listener = None;
    let mut last_err = String::new();
    for offset in 0..(if explicit || args.port == 0 { 1 } else { 20 }) {
        let addr: SocketAddr = format!("{}:{}", args.host, args.port.saturating_add(offset))
            .parse()
            .map_err(err)?;
        match TcpListener::bind(addr).await {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(e) => last_err = format!("Cannot listen on {addr}: {e}"),
        }
    }
    let Some(listener) = listener else {
        return Err(last_err);
    };
    let port = listener.local_addr().map_err(err)?.port();
    let url = format!(
        "http://{}:{}/",
        if args.host == "0.0.0.0" {
            "localhost"
        } else {
            &args.host
        },
        port
    );
    println!("Page de conversation : {url}");
    ready(
        url.clone(),
        profile
            .as_ref()
            .map(|a| a.name.clone())
            .unwrap_or_else(|| "Assistant".into()),
    );
    if args.open && !args.gui {
        let _ = std::process::Command::new(if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        })
        .arg(&url)
        .spawn();
    }
    {
        let app = app.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(20)).await;
                let _w = app.swap.write().await;
                match bundle::sync(&exe_dir, &data_dir) {
                    Ok(Some(name)) => println!("Update installed: {name}"),
                    Ok(None) => {}
                    Err(e) => eprintln!("Update refused: {e}"),
                }
            }
        });
    }
    loop {
        let (stream, _) = listener.accept().await.map_err(err)?;
        let app = app.clone();
        tokio::spawn(async move {
            if let Err(e) = handle(stream, app).await {
                eprintln!("Request: {e}");
            }
        });
    }
}
/// Engine state, for the page.
async fn health(s: &crate::db::Settings) -> Value {
    let mut problems: Vec<String> = vec![];
    let mut degraded = false;
    let embedded_chat = s.provider == llm::EMBEDDED;
    let embedded_embed = s.embedding_endpoint.trim() == llm::EMBEDDED;
    if embedded_chat && !crate::engine::available(&s.model) {
        problems.push(format!(
            "Chat model missing: {} (expected in the models/ folder).",
            s.model
        ));
    }
    if embedded_embed && !crate::engine::available(&s.embedding_model) {
        problems.push(format!(
            "Embedding model missing: {} - word search only.",
            s.embedding_model
        ));
        degraded = true;
    }
    let chat_via_ollama = !embedded_chat && s.provider == "ollama";
    let embed_via_ollama = !embedded_embed;
    if chat_via_ollama || embed_via_ollama {
        // Both go through the Ollama embedding server.
        let base = s
            .embedding_endpoint
            .trim()
            .trim_end_matches('/')
            .to_string();
        let tags = match llm::client() {
            Ok(c) => c
                .get(format!("{base}/api/tags"))
                .timeout(Duration::from_secs(6))
                .send()
                .await
                .ok(),
            Err(_) => None,
        };
        match tags {
            Some(r) if r.status().is_success() => {
                let v: Value = r.json().await.unwrap_or(json!({}));
                let names: Vec<String> = v["models"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .filter_map(|m| m["name"].as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let has = |m: &str| {
                    names.iter().any(|n| {
                        n == m || n.trim_end_matches(":latest") == m || *n == format!("{m}:latest")
                    })
                };
                if chat_via_ollama && !has(&s.model) {
                    problems.push(format!(
                        "Chat model \"{}\" missing from Ollama: `ollama pull {}`.",
                        s.model, s.model
                    ));
                }
                if embed_via_ollama && !has(&s.embedding_model) {
                    problems.push(format!("Embedding model \"{}\" missing from Ollama: `ollama pull {}` - word search only until then.", s.embedding_model, s.embedding_model));
                    degraded = true;
                }
            }
            _ => {
                if chat_via_ollama {
                    problems.push(format!(
                        "Ollama is not answering on {base}: answers will fail until it starts."
                    ));
                } else {
                    problems.push(format!(
                        "Ollama is not answering on {base}: word search only."
                    ));
                    degraded = true;
                }
            }
        }
    }
    json!({"ok": problems.is_empty(), "degraded": degraded, "problems": problems, "engine": if embedded_chat { "embedded" } else { s.provider.as_str() }, "model": s.model, "embedding_model": s.embedding_model})
}
/// Checks Ollama and pulls the missing models.
async fn prepare_ollama(db: &Db) {
    let Ok(s) = db.settings() else { return };
    let Ok(models) = llm::models(&s).await else {
        eprintln!("Ollama unreachable on {}: search will be lexical and answers will fail until it starts.", s.endpoint);
        return;
    };
    let names: Vec<String> = models
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|m| m["name"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    let has = |m: &str| {
        names
            .iter()
            .any(|n| n == m || n.trim_end_matches(":latest") == m || n == &format!("{m}:latest"))
    };
    let wanted: Vec<String> = [
        (s.provider.as_str() != llm::EMBEDDED, s.model.clone()),
        (
            s.embedding_endpoint.trim() != llm::EMBEDDED,
            s.embedding_model.clone(),
        ),
    ]
    .into_iter()
    .filter(|(via_ollama, _)| *via_ollama)
    .map(|(_, m)| m)
    .collect();
    for model in wanted {
        if has(&model) {
            continue;
        }
        println!("Downloading model {model}…");
        let base = s.endpoint.trim_end_matches('/');
        let Ok(client) = llm::client() else { continue };
        match client
            .post(format!("{base}/api/pull"))
            .json(&json!({"model": model, "stream": false}))
            .timeout(Duration::from_secs(3600))
            .send()
            .await
        {
            Ok(r) if r.status().is_success() => println!("Model {model} ready."),
            Ok(r) => eprintln!("Download of {model} refused: {}", r.status()),
            Err(e) => eprintln!("Cannot download {model}: {e}"),
        }
    }
}
struct Request {
    method: String,
    path: String,
    query: HashMap<String, String>,
    body: Vec<u8>,
}
async fn read_request(reader: &mut BufReader<TcpStream>) -> Res<Request> {
    let mut line = String::new();
    reader.read_line(&mut line).await.map_err(err)?;
    let mut parts = line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let target = parts.next().unwrap_or("/").to_string();
    let mut length = 0usize;
    loop {
        let mut h = String::new();
        reader.read_line(&mut h).await.map_err(err)?;
        let h = h.trim_end();
        if h.is_empty() {
            break;
        }
        if let Some(v) = h
            .strip_prefix("Content-Length:")
            .or_else(|| h.strip_prefix("content-length:"))
        {
            length = v.trim().parse().unwrap_or(0);
        }
    }
    if length > 2_000_000 {
        return Err("Corps trop volumineux".into());
    }
    let mut body = vec![0u8; length];
    reader.read_exact(&mut body).await.map_err(err)?;
    let (path, qs) = target.split_once('?').unwrap_or((&target, ""));
    let query = qs
        .split('&')
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            p.split_once('=')
                .map(|(k, v)| (k.to_string(), percent_decode(v)))
        })
        .collect();
    Ok(Request {
        method,
        path: path.to_string(),
        query,
        body,
    })
}
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}
async fn respond(w: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> Res<()> {
    let head = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    );
    w.write_all(head.as_bytes()).await.map_err(err)?;
    w.write_all(body).await.map_err(err)?;
    w.flush().await.map_err(err)
}
async fn json_ok(w: &mut TcpStream, v: Value) -> Res<()> {
    respond(
        w,
        "200 OK",
        "application/json; charset=utf-8",
        v.to_string().as_bytes(),
    )
    .await
}
async fn json_err(w: &mut TcpStream, status: &str, msg: &str) -> Res<()> {
    respond(
        w,
        status,
        "application/json; charset=utf-8",
        json!({"error": msg}).to_string().as_bytes(),
    )
    .await
}
async fn handle(stream: TcpStream, app: Arc<App>) -> Res<()> {
    let mut reader = BufReader::new(stream);
    let req = read_request(&mut reader).await?;
    let mut w = reader.into_inner();
    let _r = app.swap.read().await;
    match (req.method.as_str(), req.path.as_str()) {
        ("GET", "/") | ("GET", "/index.html") => {
            respond(
                &mut w,
                "200 OK",
                "text/html; charset=utf-8",
                PAGE.as_bytes(),
            )
            .await
        }
        ("GET", "/api/config") => {
            let s = app.db.settings()?;
            let a = assistant::get(&app.db, &s.active_assistant)?.unwrap_or_default();
            json_ok(&mut w, json!({"name": a.name, "welcome": a.welcome, "avatar": a.avatar, "theme": a.theme, "show_sources": a.show_sources})).await
        }
        ("GET", "/api/health") => {
            let s = app.db.settings()?;
            json_ok(&mut w, health(&s).await).await
        }
        ("POST", "/api/conversation") => {
            let s = app.db.settings()?;
            let id = pipeline::new_conversation(&app.db, Some(&s.active_assistant))?;
            json_ok(&mut w, json!({"id": id})).await
        }
        ("GET", "/api/messages") => {
            let id = req.query.get("id").cloned().unwrap_or_default();
            if !valid_id(&id) {
                return json_err(&mut w, "400 Bad Request", "identifiant invalide").await;
            }
            // No SQLite connection may cross an await: it is not Send.
            let rows = {
                let c = app.db.conn()?;
                let mut st = c.prepare("SELECT role,content,sources FROM messages WHERE conversation_id=?1 ORDER BY rowid").map_err(err)?;
                let rows = st
                    .query_map([&id], |r| {
                        let sources: String = r.get(2)?;
                        Ok(json!({"role": r.get::<_, String>(0)?, "content": r.get::<_, String>(1)?, "sources": serde_json::from_str::<Value>(&sources).unwrap_or(json!([]))}))
                    })
                    .map_err(err)?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(err)?;
                rows
            };
            json_ok(&mut w, Value::Array(rows)).await
        }
        ("POST", "/api/chat") => chat(&mut w, &req, &app).await,
        _ => json_err(&mut w, "404 Not Found", "introuvable").await,
    }
}
fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}
async fn chat(w: &mut TcpStream, req: &Request, app: &Arc<App>) -> Res<()> {
    let body: Value = serde_json::from_slice(&req.body).unwrap_or(json!({}));
    let question = body["question"].as_str().unwrap_or("").trim().to_string();
    let mut conversation = body["conversation_id"].as_str().unwrap_or("").to_string();
    if question.is_empty() {
        return json_err(w, "400 Bad Request", "question vide").await;
    }
    let s = app.db.settings()?;
    if conversation.is_empty() || !valid_id(&conversation) {
        conversation = pipeline::new_conversation(&app.db, Some(&s.active_assistant))?;
    } else {
        let known: i64 = {
            let c = app.db.conn()?;
            c.query_row(
                "SELECT count(*) FROM conversations WHERE id=?1",
                [&conversation],
                |r| r.get(0),
            )
            .map_err(err)?
        };
        if known == 0 {
            conversation = pipeline::new_conversation(&app.db, Some(&s.active_assistant))?;
        }
    }
    let profile = assistant::get(&app.db, &s.active_assistant)?;
    let effective = assistant::effective(&s, profile.as_ref())?;
    w.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n").await.map_err(err)?;
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(String, Value)>();
    let emit_tx = tx.clone();
    let emit = move |kind: &str, payload: Value| {
        let _ = emit_tx.send((kind.to_string(), payload));
    };
    let db = app.db.clone();
    let conv = conversation.clone();
    let gen_lock = app.generation_shared.lock().await;
    let cancel = Arc::new(AtomicBool::new(false));
    let worker = tokio::spawn(async move {
        let r = pipeline::answer(
            &db,
            &effective,
            profile.as_ref(),
            &conv,
            &question,
            "hybrid",
            false,
            cancel,
            &emit,
        )
        .await;
        let _ = tx.send((
            "__done".into(),
            match r {
                Ok(v) => v,
                Err(e) => json!({"error": e}),
            },
        ));
    });
    let _ = tx_event(w, "conversation", json!({"id": conversation})).await;
    while let Some((kind, payload)) = rx.recv().await {
        if kind == "__done" {
            let event = if payload["error"].is_null() {
                "done"
            } else {
                "error"
            };
            let _ = tx_event(w, event, payload).await;
            break;
        }
        if tx_event(w, &kind, payload).await.is_err() {
            break;
        }
    }
    let _ = worker.await;
    drop(gen_lock);
    Ok(())
}
async fn tx_event(w: &mut TcpStream, event: &str, payload: Value) -> Res<()> {
    let frame = format!("event: {event}\ndata: {}\n\n", payload);
    w.write_all(frame.as_bytes()).await.map_err(err)?;
    w.flush().await.map_err(err)
}
