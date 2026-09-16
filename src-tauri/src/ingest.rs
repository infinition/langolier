use crate::{
    db::{err, now, Db, Res, Settings},
    llm, rag,
};
use rusqlite::params;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use tokio::process::Command;
pub const TYPES: &[&str] = &[
    "txt", "md", "py", "rs", "js", "ts", "tsx", "jsx", "json", "csv", "toml", "yaml", "yml",
    "html", "css", "sql", "r", "jl", "ipynb", "pdf", "srt", "vtt", "mp4", "mkv", "mov", "webm",
    "avi", "m4v", "mp3", "wav", "m4a", "flac", "ogg", "docx", "pptx", "xlsx", "odt", "odp", "ods",
];
fn media(ext: &str) -> bool {
    [
        "mp4", "mkv", "mov", "webm", "avi", "m4v", "mp3", "wav", "m4a", "flac", "ogg",
    ]
    .contains(&ext)
}
pub fn binary(name: &str) -> PathBuf {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    let filename = format!("{name}{suffix}");
    let mut candidates = vec![
        PathBuf::from("/opt/homebrew/bin").join(&filename),
        PathBuf::from("/usr/local/bin").join(&filename),
    ];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(p) = exe.parent() {
            candidates.insert(0, p.join("tools").join(&filename));
        }
    }
    if let Ok(programs) = std::env::var("ProgramFiles") {
        candidates.push(
            PathBuf::from(programs)
                .join("Tesseract-OCR")
                .join(&filename),
        );
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("Langolier/tools").join(&filename));
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(PathBuf::from(home).join(".local/bin").join(&filename));
    }
    for p in &candidates {
        if p.is_file() {
            return p.clone();
        }
    }
    PathBuf::from(filename)
}
pub async fn command(name: &str, args: &[String], timeout: u64) -> Res<String> {
    let out = tokio::time::timeout(
        Duration::from_secs(timeout),
        Command::new(binary(name))
            .args(args)
            .env("LC_ALL", "C")
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| format!("{name}: timed out"))?
    .map_err(|e| {
        format!("{name} indisponible : {e}. Lancez le script d'installation des moteurs.")
    })?;
    if !out.status.success() {
        return Err(format!(
            "{name} : {}",
            String::from_utf8_lossy(&out.stderr)
                .chars()
                .rev()
                .take(1800)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        ));
    }
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}
pub fn queue(db: &Db, paths: Vec<String>) -> Res<Value> {
    let mut added = 0;
    let mut skipped = vec![];
    let c = db.conn()?;
    for input in paths {
        let p = PathBuf::from(&input);
        let files = if p.is_dir() {
            walkdir::WalkDir::new(&p)
                .follow_links(false)
                .into_iter()
                .filter_entry(|e| {
                    !matches!(
                        e.file_name().to_str(),
                        Some(".git" | "node_modules" | "target" | ".venv" | "venv" | "__pycache__")
                    )
                })
                .filter_map(Result::ok)
                .filter(|e| e.file_type().is_file())
                .map(|e| e.into_path())
                .collect::<Vec<_>>()
        } else {
            vec![p]
        };
        for p in files {
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            if !TYPES.contains(&ext.as_str()) {
                skipped.push(p.display().to_string());
                continue;
            }
            let p = p.canonicalize().map_err(err)?;
            let source = p.to_string_lossy().to_string();
            let exists: i64 = c
                .query_row(
                    "SELECT count(*) FROM documents WHERE source=?1",
                    [&source],
                    |r| r.get(0),
                )
                .map_err(err)?;
            if exists > 0 {
                skipped.push(source);
                continue;
            }
            let name = p.file_name().unwrap_or_default().to_string_lossy();
            let id = uuid::Uuid::new_v4().to_string();
            let bytes = p.metadata().map_err(err)?.len();
            c.execute("INSERT INTO documents(id,name,source,kind,bytes,created,updated)VALUES(?1,?2,?3,?4,?5,?6,?6)",params![id,name,source,ext,bytes,now()]).map_err(err)?;
            added += 1;
        }
    }
    Ok(json!({"added":added,"skipped":skipped}))
}
pub fn paste(db: &Db, title: &str, text: &str) -> Res<Value> {
    if text.trim().is_empty() {
        return Err("The text is empty.".into());
    }
    if text.len() > 20_000_000 {
        return Err("Text capped at 20 MB per source.".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    let path = db.root.join("media").join(format!("{id}.md"));
    std::fs::write(&path, text).map_err(err)?;
    let result = queue(db, vec![path.to_string_lossy().to_string()])?;
    db.conn()?
        .execute(
            "UPDATE documents SET name=?1 WHERE source=?2",
            params![
                if title.trim().is_empty() {
                    "Pasted text"
                } else {
                    title.trim()
                },
                path.to_string_lossy()
            ],
        )
        .map_err(err)?;
    Ok(result)
}
pub fn queue_url(db: &Db, url: &str) -> Res<Value> {
    let u = reqwest::Url::parse(url.trim()).map_err(err)?;
    if !matches!(u.scheme(), "http" | "https") || !u.username().is_empty() || u.password().is_some()
    {
        return Err("An HTTP or HTTPS video link without credentials is required.".into());
    }
    let c = db.conn()?;
    let count: i64 = c
        .query_row(
            "SELECT count(*) FROM documents WHERE source=?1",
            [u.as_str()],
            |r| r.get(0),
        )
        .map_err(err)?;
    if count > 0 {
        return Ok(json!({"added":0,"skipped":[url]}));
    }
    let id = uuid::Uuid::new_v4().to_string();
    c.execute(
        "INSERT INTO documents(id,name,source,kind,created,updated)VALUES(?1,?2,?2,'url',?3,?3)",
        params![id, u.as_str(), now()],
    )
    .map_err(err)?;
    Ok(json!({"added":1,"skipped":[]}))
}
#[derive(Clone)]
pub struct Section {
    pub text: String,
    pub locator: String,
}
pub fn notebook(text: &str) -> Res<Vec<Section>> {
    let v: Value = serde_json::from_str(text).map_err(err)?;
    let cells = v["cells"].as_array().ok_or("Invalid notebook: no cells")?;
    Ok(cells
        .iter()
        .enumerate()
        .filter_map(|(i, c)| {
            let source = if let Some(a) = c["source"].as_array() {
                a.iter().filter_map(Value::as_str).collect::<String>()
            } else {
                c["source"].as_str().unwrap_or("").into()
            };
            if source.trim().is_empty() {
                None
            } else {
                Some(Section {
                    text: source,
                    locator: format!(
                        "Cellule {} ({})",
                        i + 1,
                        c["cell_type"].as_str().unwrap_or("texte")
                    ),
                })
            }
        })
        .collect())
}
pub fn subtitles(text: &str) -> Vec<Section> {
    text.replace("\r\n", "\n")
        .split("\n\n")
        .filter_map(|block| {
            let mut lines = block.lines();
            let mut stamp = None;
            let mut content = vec![];
            for l in lines.by_ref() {
                if l.contains("-->") {
                    stamp = Some(l.to_string())
                } else if stamp.is_some() {
                    content.push(l)
                }
            }
            stamp.map(|locator| Section {
                text: content.join(" "),
                locator,
            })
        })
        .collect()
}
pub fn text_sections(text: &str) -> Vec<Section> {
    let mut sections = vec![];
    let mut block = String::new();
    let mut label = "Start".to_string();
    let mut start = 1;
    for (i, line) in text.lines().enumerate() {
        if line.starts_with('#') && !block.trim().is_empty() {
            sections.push(Section {
                text: std::mem::take(&mut block),
                locator: format!("{} · ligne {}", label, start),
            });
            label = line.trim_start_matches('#').trim().to_string();
            start = i + 1;
        }
        block.push_str(line);
        block.push('\n')
    }
    if !block.trim().is_empty() {
        sections.push(Section {
            text: block,
            locator: format!("{} · ligne {}", label, start),
        })
    }
    sections
}
fn hash_file(p: &Path) -> Res<String> {
    let mut f = std::fs::File::open(p).map_err(err)?;
    let mut hash = Sha256::new();
    let mut buf = [0u8; 65536];
    loop {
        let n = f.read(&mut buf).map_err(err)?;
        if n == 0 {
            break;
        }
        hash.update(&buf[..n])
    }
    Ok(format!("{:x}", hash.finalize()))
}
async fn transcribe(
    db: &Db,
    id: &str,
    p: &Path,
    s: &Settings,
) -> Res<(Vec<Section>, Option<String>)> {
    if !Path::new(&s.whisper_model).is_file() {
        return Err("Whisper model missing. Install the engines, then point to the ggml file under Engines.".into());
    }
    db.stage(id, "Audio extraction · FFmpeg")?;
    let wav = db.root.join("media").join(format!("{id}.wav"));
    command(
        "ffmpeg",
        &[
            "-nostdin".into(),
            "-y".into(),
            "-i".into(),
            p.display().to_string(),
            "-vn".into(),
            "-ar".into(),
            "16000".into(),
            "-ac".into(),
            "1".into(),
            "-c:a".into(),
            "pcm_s16le".into(),
            wav.display().to_string(),
        ],
        7200,
    )
    .await?;
    db.stage(id, "Transcription · automatic language detection")?;
    let out = db.root.join("media").join(format!("{id}-transcript"));
    let result = command(
        "whisper-cli",
        &[
            "-m".into(),
            s.whisper_model.clone(),
            "-f".into(),
            wav.display().to_string(),
            "-l".into(),
            "auto".into(),
            "-oj".into(),
            "-of".into(),
            out.display().to_string(),
            "-t".into(),
            "4".into(),
        ],
        14400,
    )
    .await;
    let _ = std::fs::remove_file(&wav);
    result?;
    let v: Value =
        serde_json::from_str(&std::fs::read_to_string(out.with_extension("json")).map_err(err)?)
            .map_err(err)?;
    let language = v["result"]["language"].as_str().map(String::from);
    let parts = v["transcription"]
        .as_array()
        .ok_or("Empty Whisper transcription")?;
    let sections = parts
        .iter()
        .map(|p| Section {
            text: p["text"].as_str().unwrap_or("").trim().to_string(),
            locator: format!(
                "{} --> {}",
                p["timestamps"]["from"].as_str().unwrap_or("00:00"),
                p["timestamps"]["to"].as_str().unwrap_or("00:00")
            ),
        })
        .collect::<Vec<_>>();
    Ok((merge_segments(sections), language))
}
fn merge_segments(sections: Vec<Section>) -> Vec<Section> {
    let mut out = vec![];
    let mut block = String::new();
    let mut start = String::new();
    let mut end = String::new();
    for s in sections {
        if block.is_empty() {
            start = s.locator.split(" --> ").next().unwrap_or("").into()
        }
        end = s.locator.split(" --> ").last().unwrap_or("").into();
        block.push_str(&s.text);
        block.push(' ');
        if block.chars().count() >= 900 {
            out.push(Section {
                text: std::mem::take(&mut block),
                locator: format!("{start} --> {end}"),
            })
        }
    }
    if !block.trim().is_empty() {
        out.push(Section {
            text: block,
            locator: format!("{start} --> {end}"),
        })
    }
    out
}
fn blank(pages: &[String]) -> bool {
    pages.iter().all(|p| p.trim().is_empty())
}
async fn pdftotext_pages(path: &Path) -> Res<Vec<String>> {
    let out = command(
        "pdftotext",
        &[
            "-layout".into(),
            "-enc".into(),
            "UTF-8".into(),
            path.display().to_string(),
            "-".into(),
        ],
        900,
    )
    .await?;
    let mut pages: Vec<String> = out.split('\u{c}').map(|p| p.to_string()).collect();
    if pages.last().is_some_and(|p| p.trim().is_empty()) {
        pages.pop();
    }
    Ok(pages)
}
async fn pdf_sections(db: &Db, id: &str, path: &Path) -> Res<Vec<Section>> {
    let file = path.to_path_buf();
    // pdf-extract panics on some malformed content streams, so a join failure counts
    // as a failed extraction rather than aborting the whole source.
    let extracted =
        tokio::task::spawn_blocking(move || pdf_extract::extract_text_by_pages(&file)).await;
    let native = match extracted {
        Ok(Ok(pages)) if !pages.is_empty() && !blank(&pages) => Some(pages),
        _ => None,
    };
    let mut pages = match native {
        Some(pages) => pages,
        None => {
            db.stage(id, "Text extraction · pdftotext")?;
            match pdftotext_pages(path).await {
                Ok(pages) if !pages.is_empty() => pages,
                _ => {
                    let info = command("pdfinfo", &[path.display().to_string()], 60).await?;
                    let count = info
                        .lines()
                        .find_map(|line| {
                            line.strip_prefix("Pages:")
                                .and_then(|s| s.trim().parse::<usize>().ok())
                        })
                        .ok_or("Cannot determine the PDF page count.")?;
                    if count > 10000 {
                        return Err("PDF capped at 10,000 pages per source.".into());
                    }
                    vec![String::new(); count]
                }
            }
        }
    };
    let count = pages.len();
    if count > 10000 {
        return Err("PDF capped at 10,000 pages per source.".into());
    }
    let tessdata = db.root.join("models/tessdata");
    let mut language_args = if tessdata.join("eng.traineddata").is_file()
        && tessdata.join("fra.traineddata").is_file()
    {
        vec![
            "--tessdata-dir".to_string(),
            tessdata.display().to_string(),
            "-l".into(),
            "eng+fra".into(),
        ]
    } else {
        let languages = command("tesseract", &["--list-langs".into()], 20).await;
        let langs = languages.unwrap_or_default();
        vec![
            "-l".into(),
            if langs.lines().any(|l| l.trim() == "fra") {
                "eng+fra".into()
            } else {
                "eng".into()
            },
        ]
    };
    language_args.extend(["--psm".into(), "3".into()]);
    let mut sections = Vec::with_capacity(count);
    for (index, text) in pages.iter_mut().enumerate() {
        let scanned = text.chars().filter(|c| c.is_alphanumeric()).count() < 30;
        if scanned {
            db.stage(id, &format!("OCR · page {}/{}", index + 1, count))?;
            let prefix = db.root.join("media").join(format!("{id}-ocr-page"));
            let image = prefix.with_extension("png");
            command(
                "pdftoppm",
                &[
                    "-f".into(),
                    (index + 1).to_string(),
                    "-l".into(),
                    (index + 1).to_string(),
                    "-singlefile".into(),
                    "-scale-to".into(),
                    "2600".into(),
                    "-png".into(),
                    path.display().to_string(),
                    prefix.display().to_string(),
                ],
                180,
            )
            .await?;
            let mut args = vec![image.display().to_string(), "stdout".into()];
            args.extend(language_args.clone());
            let recognized = command("tesseract", &args, 240).await;
            let _ = std::fs::remove_file(image);
            let recognized = recognized.map_err(|e| format!("OCR page {} : {e}", index + 1))?;
            if !recognized.trim().is_empty() {
                *text = recognized;
            }
        }
        sections.push(Section {
            text: std::mem::take(text),
            locator: format!("Page {}{}", index + 1, if scanned { " · OCR" } else { "" }),
        });
    }
    Ok(sections)
}
pub(crate) async fn process(db: &Db, id: &str, source: &str, kind: &str, name: &str) -> Res<()> {
    let s = db.settings()?;
    let mut name = name.to_string();
    let path = if kind == "url" {
        db.stage(id, "Downloading media · yt-dlp")?;
        let template = db.root.join("media").join(format!("{id}.%(ext)s"));
        let result = command(
            "yt-dlp",
            &[
                "--ignore-config".into(),
                "--no-playlist".into(),
                "--max-filesize".into(),
                "4G".into(),
                "--no-progress".into(),
                "--restrict-filenames".into(),
                "-f".into(),
                "bestaudio/best".into(),
                "--print".into(),
                "title".into(),
                "--print".into(),
                "after_move:filepath".into(),
                "-o".into(),
                template.display().to_string(),
                "--".into(),
                source.into(),
            ],
            7200,
        )
        .await?;
        let mut lines = result.lines().filter(|l| !l.trim().is_empty());
        let first = lines
            .next()
            .ok_or("No media downloaded")?
            .trim()
            .to_string();
        let last = lines
            .next_back()
            .map(|l| l.trim().to_string())
            .unwrap_or_else(|| first.clone());
        if last != first {
            // The title replaces the URL only when it was not customised.
            let title: String = first.chars().take(180).collect();
            db.conn()?
                .execute(
                    "UPDATE documents SET name=?2 WHERE id=?1 AND name=source",
                    params![id, title],
                )
                .map_err(err)?;
            name = title;
        }
        PathBuf::from(last)
    } else {
        PathBuf::from(source)
    };
    db.stage(id, "Reading and SHA-256 fingerprint")?;
    let p = path.clone();
    let hash = tokio::task::spawn_blocking(move || hash_file(&p))
        .await
        .map_err(err)??;
    {
        let c = db.conn()?;
        let duplicate: Option<String> = c
            .query_row(
                "SELECT name FROM documents WHERE hash=?1 AND id!=?2",
                params![hash, id],
                |r| r.get(0),
            )
            .ok();
        if let Some(other) = duplicate {
            return Err(format!("Doublon de contenu : {other}"));
        }
        c.execute(
            "UPDATE documents SET hash=?2 WHERE id=?1",
            params![id, hash],
        )
        .map_err(err)?;
    }
    let (sections, language) = if kind == "url" || media(kind) {
        transcribe(db, id, &path, &s).await?
    } else {
        if path.metadata().map_err(err)?.len() > 256 * 1024 * 1024 {
            return Err("Document capped at 256 MB per file.".into());
        }
        let p = path.clone();
        let k = kind.to_string();
        let secs = if kind == "pdf" {
            pdf_sections(db, id, &path).await?
        } else {
            tokio::task::spawn_blocking(move || -> Res<Vec<Section>> {
                if p.metadata().map_err(err)?.len() > 256 * 1024 * 1024 {
                    return Err("Document capped at 256 MB per file.".into());
                }
                if k == "pdf" {
                    let pages = pdf_extract::extract_text_by_pages(&p).map_err(err)?;
                    Ok(pages
                        .into_iter()
                        .enumerate()
                        .map(|(i, text)| Section {
                            text,
                            locator: format!("Page {}", i + 1),
                        })
                        .collect())
                } else if crate::office::is_office(&k) {
                    crate::office::sections(&k, &std::fs::read(&p).map_err(err)?)
                } else {
                    let t = std::fs::read_to_string(&p).map_err(err)?;
                    if k == "ipynb" {
                        notebook(&t)
                    } else if k == "srt" || k == "vtt" {
                        Ok(merge_segments(subtitles(&t)))
                    } else {
                        Ok(text_sections(&t))
                    }
                }
            })
            .await
            .map_err(err)??
        };
        let sample = secs
            .iter()
            .map(|x| x.text.as_str())
            .collect::<Vec<_>>()
            .join(" ")
            .chars()
            .take(10000)
            .collect::<String>();
        let language = whatlang::detect(&sample).map(|v| v.lang().code().to_string());
        (secs, language)
    };
    let mut chunks = vec![];
    for section in sections {
        for (i, text) in rag::chunk(&section.text, 1400, 200).into_iter().enumerate() {
            chunks.push((text, format!("{} · extrait {}", section.locator, i + 1)))
        }
    }
    if chunks.is_empty() {
        return Err(
            "No usable text after extraction. Check that the source holds readable text.".into(),
        );
    }
    {
        let mut c = db.conn()?;
        let tx = c.transaction().map_err(err)?;
        tx.execute("DELETE FROM chunks WHERE doc_id=?1", [id])
            .map_err(err)?;
        for (i, (text, locator)) in chunks.iter().enumerate() {
            tx.execute(
                "INSERT INTO chunks(doc_id,ordinal,text,locator) VALUES(?1,?2,?3,?4)",
                params![id, i, text, locator],
            )
            .map_err(err)?;
        }
        tx.commit().map_err(err)?;
    }
    let mut warning = None;
    for (batch_no, batch) in chunks.chunks(16).enumerate() {
        db.stage(
            id,
            &format!(
                "Semantic indexing · {}/{}",
                (batch_no * 16 + batch.len()),
                chunks.len()
            ),
        )?;
        let texts = batch
            .iter()
            .map(|(t, l)| format!("Document: {name}\nSection: {l}\n{t}"))
            .collect::<Vec<_>>();
        match llm::embeddings(&s, &texts).await {
            Ok(vectors) => {
                let mut c = db.conn()?;
                let tx = c.transaction().map_err(err)?;
                for (i, v) in vectors.iter().enumerate() {
                    tx.execute("UPDATE chunks SET embedding=?3,embedding_model=?4 WHERE doc_id=?1 AND ordinal=?2",params![id,batch_no*16+i,rag::encode(v),rag::embedding_key(&s)]).map_err(err)?;
                }
                tx.commit().map_err(err)?;
            }
            Err(e) => {
                warning = Some(format!("Lexical index ready. Embeddings incomplete: {e}"));
                break;
            }
        }
    }
    db.conn()?.execute("UPDATE documents SET status='ready',stage=?2,error=?3,language=?4,updated=?5 WHERE id=?1",params![id,if warning.is_some(){"Lexical index"}else{"Hybrid index"},warning,language,now()]).map_err(err)?;
    Ok(())
}
pub async fn worker(db: Db, notify: impl Fn() + Send + 'static) {
    loop {
        if db.settings().map(|s| s.ingestion_paused).unwrap_or(false) {
            tokio::time::sleep(Duration::from_secs(2)).await;
            continue;
        }
        let next = (|| -> Res<Option<(String, String, String, String)>> {
            let mut c = db.conn()?;
            let tx = c.transaction().map_err(err)?;
            let row=tx.query_row("SELECT id,source,kind,name FROM documents WHERE status='queued' ORDER BY created LIMIT 1",[],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).ok();
            if let Some((id, _, _, _)) = &row {
                tx.execute(
                    "UPDATE documents SET status='processing',stage='Preparing' WHERE id=?1",
                    [id],
                )
                .map_err(err)?;
            }
            tx.commit().map_err(err)?;
            Ok(row)
        })();
        if let Ok(Some((id, source, kind, name))) = next {
            notify();
            let start = Instant::now();
            let result = process(&db, &id, &source, &kind, &name).await;
            let status = if result.is_ok() { "ok" } else { "error" };
            if let Err(e) = &result {
                if let Ok(c) = db.conn() {
                    let _=c.execute("UPDATE documents SET status='error',stage='Needs checking',error=?2,updated=?3 WHERE id=?1",params![id,e,now()]);
                }
            }
            let _ = db.log(
                "ingestion",
                "",
                status,
                start.elapsed().as_millis() as u64,
                0,
                None,
                &json!({"document":id,"error":result.err()}),
            );
            notify();
        } else {
            tokio::time::sleep(Duration::from_millis(800)).await
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn notebook_does_not_execute_or_ingest_output() {
        let s = r#"{"cells":[{"cell_type":"code","source":["print(1)"],"outputs":[{"text":"secret output"}]}]}"#;
        let r = notebook(s).unwrap();
        assert_eq!(r[0].text, "print(1)");
    }
    #[test]
    fn timestamps_survive() {
        let r = subtitles("WEBVTT\n\n00:00:01.000 --> 00:00:04.000\nBonjour le monde\n\n");
        assert_eq!(r[0].text, "Bonjour le monde");
        assert!(r[0].locator.contains("00:00:01"));
    }
    #[test]
    fn queue_and_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::new(temp.path()).unwrap();
        paste(&db, "Note", "Bonjour monde").unwrap();
        db.conn()
            .unwrap()
            .execute("UPDATE documents SET status='processing'", [])
            .unwrap();
        let db = Db::new(temp.path()).unwrap();
        let n: i64 = db
            .conn()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM documents WHERE status='queued'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 1);
    }

    #[tokio::test]
    #[ignore = "Requires Poppler and Tesseract"]
    async fn scanned_pdf_ocr() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::new(temp.path()).unwrap();
        let file = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/scanned-note.pdf");
        assert!(pdf_extract::extract_text(&file).unwrap().trim().is_empty());
        let start = Instant::now();
        let sections = pdf_sections(&db, "ocr-fixture", &file).await.unwrap();
        assert!(sections[0].text.to_lowercase().contains("regularization"));
        assert!(sections[0].text.to_lowercase().contains("validation"));
        assert!(sections[0].locator.contains("OCR"));
        println!(
            "OCR_TEST {}",
            json!({"pages":sections.len(),"characters":sections[0].text.len(),"elapsed_ms":start.elapsed().as_millis()})
        );
    }
}
