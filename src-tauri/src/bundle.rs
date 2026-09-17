//! The .langolier file: profile, settings and knowledge in one archive.
use crate::{
    assistant::{self, Assistant},
    db::{err, Db, Res},
};
use rusqlite::Connection;
use sha2::{Digest, Sha256};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};

pub const MAGIC: &[u8; 18] = b"LANGOLIER-BUNDLE-1";
const FOOTER: u64 = 8 + 18;

pub fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let s = s.trim_matches('-').to_string();
    let mut out = String::new();
    for part in s.split('-').filter(|p| !p.is_empty()) {
        if !out.is_empty() {
            out.push('-');
        }
        out.push_str(part);
    }
    if out.is_empty() {
        "assistant".into()
    } else {
        out
    }
}
/// Writes a profile's .langolier to dest.
pub fn write_langolier(db: &Db, a: &Assistant, dest: &Path) -> Res<u64> {
    write_langolier_with(db, a, dest, None)
}
/// Variant with forced exported settings.
pub fn write_langolier_with(
    db: &Db,
    a: &Assistant,
    dest: &Path,
    forced: Option<&crate::db::Settings>,
) -> Res<u64> {
    let base = db.settings()?;
    let s = match forced {
        Some(f) => f.clone(),
        None => assistant::effective(&base, Some(a))?,
    };
    let scope = assistant::scope_docs(db, Some(a))?;
    if dest.exists() {
        std::fs::remove_file(dest).map_err(err)?;
    }
    {
        let c = db.conn()?;
        c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(err)?;
        c.execute("VACUUM INTO ?1", [dest.to_string_lossy()])
            .map_err(err)?;
    }
    let c = Connection::open(dest).map_err(err)?;
    c.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA foreign_keys=ON;")
        .map_err(err)?;
    if let Some(ids) = &scope {
        c.execute(
            "DELETE FROM documents WHERE id NOT IN (SELECT value FROM json_each(?1))",
            [serde_json::to_string(ids).map_err(err)?],
        )
        .map_err(err)?;
    }
    c.execute_batch(
        "DELETE FROM documents WHERE status!='ready';
         UPDATE documents SET watch_id=NULL;
         DELETE FROM conversations; DELETE FROM messages; DELETE FROM runs; DELETE FROM evaluations;
         DELETE FROM watch_files; DELETE FROM watches;
         DELETE FROM telegram_chats; DELETE FROM telegram_state; DELETE FROM api_keys;",
    )
    .map_err(err)?;
    c.execute("DELETE FROM assistants WHERE id!=?1", [&a.id])
        .map_err(err)?;
    // Effective profile settings; anything meaningless outside the app is cleared.
    let mut export = s.clone();
    export.active_assistant = a.id.clone();
    export.whisper_model.clear();
    export.shortcut.clear();
    if !crate::llm::is_cloud(&export.provider) {
        export.api_key.clear();
    }
    c.execute(
        "INSERT INTO settings VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET value=excluded.value",
        [serde_json::to_string(&export).map_err(err)?],
    )
    .map_err(err)?;
    c.execute_batch("VACUUM;").map_err(err)?;
    drop(c);
    Ok(std::fs::metadata(dest).map_err(err)?.len())
}
/// Payload embedded in the current executable, if any.
pub fn embedded_payload() -> Res<Option<Vec<u8>>> {
    let exe = std::env::current_exe().map_err(err)?;
    let mut f = std::fs::File::open(&exe).map_err(err)?;
    let len = f.metadata().map_err(err)?.len();
    if len < FOOTER {
        return Ok(None);
    }
    f.seek(SeekFrom::Start(len - FOOTER)).map_err(err)?;
    let mut footer = [0u8; FOOTER as usize];
    f.read_exact(&mut footer).map_err(err)?;
    if &footer[8..] != MAGIC {
        return Ok(None);
    }
    let mut b = [0u8; 8];
    b.copy_from_slice(&footer[..8]);
    let n = u64::from_le_bytes(b);
    if n > len - FOOTER {
        return Ok(None);
    }
    f.seek(SeekFrom::Start(len - FOOTER - n)).map_err(err)?;
    let mut payload = vec![0u8; n as usize];
    f.read_exact(&mut payload).map_err(err)?;
    Ok(Some(payload))
}
pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn validate(path: &Path) -> Res<Assistant> {
    let c = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(err)?;
    let n: i64 = c
        .query_row("SELECT count(*) FROM assistants", [], |r| r.get(0))
        .map_err(|_| "This file is not a Langolier bundle.".to_string())?;
    if n != 1 {
        return Err(format!(
            "A bundle must hold exactly one profile, this one holds {n}."
        ));
    }
    let raw: String = c
        .query_row("SELECT name FROM assistants", [], |r| r.get(0))
        .map_err(err)?;
    Ok(Assistant {
        name: raw,
        ..Default::default()
    })
}
/// Installs a bundle as the server's working database, keeping conversations.
pub fn install(bundle_bytes: &[u8], data_dir: &Path) -> Res<()> {
    std::fs::create_dir_all(data_dir.join("media")).map_err(err)?;
    let staged = data_dir.join("incoming.langolier");
    std::fs::write(&staged, bundle_bytes).map_err(err)?;
    validate(&staged)?;
    let live = data_dir.join("langolier.sqlite3");
    // Pooled connections keep the file open, and Windows will not replace it.
    crate::db::close_pooled(data_dir);
    if live.exists() {
        // Conversations are the only thing the server produced.
        let c = Connection::open(&live).map_err(err)?;
        c.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .map_err(err)?;
        drop(c);
        let c = Connection::open(&staged).map_err(err)?;
        crate::assistant::migrate(&c)?;
        c.execute("ATTACH DATABASE ?1 AS old", [live.to_string_lossy()])
            .map_err(err)?;
        // Administration belongs to the deployment, not to the content: an
        // incoming bundle only overrides it when it brings its own secret.
        let old_has_admin: i64 = c
            .query_row("SELECT count(*) FROM old.pragma_table_info('assistants') WHERE name='admin_secret'", [], |r| r.get(0))
            .map_err(err)?;
        if old_has_admin > 0 {
            c.execute_batch(
                "UPDATE assistants SET
                   admin_enabled=(SELECT admin_enabled FROM old.assistants LIMIT 1),
                   admin_secret=(SELECT admin_secret FROM old.assistants LIMIT 1),
                   admin_import=(SELECT admin_import FROM old.assistants LIMIT 1),
                   admin_restore=(SELECT admin_restore FROM old.assistants LIMIT 1)
                 WHERE (admin_enabled=0 OR admin_secret='')
                   AND (SELECT count(*) FROM old.assistants WHERE admin_secret!='')>0;",
            )
            .map_err(err)?;
        }
        c.execute_batch(
            "INSERT OR IGNORE INTO conversations SELECT * FROM old.conversations;
             INSERT OR IGNORE INTO messages SELECT * FROM old.messages;
             INSERT OR IGNORE INTO runs SELECT * FROM old.runs;
             INSERT OR IGNORE INTO telegram_chats SELECT * FROM old.telegram_chats;
             INSERT OR IGNORE INTO telegram_state SELECT * FROM old.telegram_state;
             DELETE FROM telegram_chats WHERE assistant_id NOT IN (SELECT id FROM assistants);
             DETACH DATABASE old;",
        )
        .map_err(err)?;
        drop(c);
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(data_dir.join(format!("langolier.sqlite3{suffix}")));
        }
    }
    std::fs::rename(&staged, &live).map_err(err)?;
    // A bundle written by an older version lacks newer columns: migrate now,
    // not at the next start.
    Db::new(data_dir)?;
    std::fs::write(data_dir.join("bundle.stamp"), digest(bundle_bytes)).map_err(err)?;
    Ok(())
}
pub fn installed_stamp(data_dir: &Path) -> String {
    std::fs::read_to_string(data_dir.join("bundle.stamp")).unwrap_or_default()
}
/// A bundle the server could run: a file next to the launcher, an entry of
/// the store, or the payload embedded in the executable.
pub struct Candidate {
    pub name: String,
    pub path: Option<PathBuf>,
    pub modified: std::time::SystemTime,
}
pub fn candidates(exe_dir: &Path, data_dir: Option<&Path>) -> Res<Vec<Candidate>> {
    let mut out = vec![];
    if embedded_payload()?.is_some() {
        out.push(Candidate {
            name: "(embedded)".into(),
            path: None,
            modified: std::time::UNIX_EPOCH,
        });
    }
    let mut dirs = vec![exe_dir.to_path_buf()];
    if let Some(d) = data_dir {
        dirs.push(store_dir(d));
    }
    for dir in dirs {
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in rd.filter_map(Result::ok) {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("langolier") {
                continue;
            }
            let Ok(m) = p.metadata() else { continue };
            out.push(Candidate {
                name: p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
                modified: m.modified().unwrap_or(std::time::UNIX_EPOCH),
                path: Some(p),
            });
        }
    }
    out.sort_by_key(|c| c.modified);
    Ok(out)
}
pub fn read_candidate(c: &Candidate) -> Res<Vec<u8>> {
    match &c.path {
        Some(p) => std::fs::read(p).map_err(err),
        None => embedded_payload()?.ok_or_else(|| "Embedded payload missing.".to_string()),
    }
}
/// Newest candidate wins. Every install lands in the store, so the dropdown
/// of the chat page shows the kit's own bundle as well as later imports.
pub fn sync(exe_dir: &Path, data_dir: &Path) -> Res<Option<String>> {
    let cands = candidates(exe_dir, Some(data_dir))?;
    let Some(c) = cands.last() else {
        return Ok(None);
    };
    let bytes = read_candidate(c)?;
    if digest(&bytes) == installed_stamp(data_dir) {
        return Ok(None);
    }
    let from_kit = c
        .path
        .as_ref()
        .is_none_or(|p| p.parent() != Some(store_dir(data_dir).as_path()));
    install(&bytes, data_dir)?;
    archive(data_dir, &bytes, from_kit)?;
    Ok(Some(c.name.clone()))
}
/// Bumps the mtime so the file becomes the newest candidate. Windows wants a
/// writable handle for that.
fn touch(path: &Path) -> std::io::Result<()> {
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)?
        .set_modified(std::time::SystemTime::now())
}
pub fn store_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("bundles")
}
const STORE_KEEP: usize = 20;
/// One file per distinct bundle: <slug>-<timestamp>-<digest8>[.kit].langolier
/// A `.kit` marker names the bundle the kit shipped with, which is never deleted.
pub fn archive(data_dir: &Path, bytes: &[u8], from_kit: bool) -> Res<PathBuf> {
    let dir = store_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(err)?;
    let d = digest(bytes);
    let short = &d[..8];
    if let Some(existing) = store_entries(data_dir)?
        .into_iter()
        .find(|e| e.digest8 == short)
    {
        return Ok(existing.path);
    }
    let staged = dir.join(format!("incoming-{}.langolier", std::process::id()));
    std::fs::write(&staged, bytes).map_err(err)?;
    let name = validate(&staged)
        .map(|a| slug(&a.name))
        .unwrap_or_else(|_| "bundle".into());
    let stamp = chrono_stamp();
    let file = dir.join(format!(
        "{name}-{stamp}-{short}{}.langolier",
        if from_kit { ".kit" } else { "" }
    ));
    std::fs::rename(&staged, &file).map_err(err)?;
    prune(data_dir)?;
    Ok(file)
}
fn chrono_stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Civil date from a Unix timestamp, UTC, no chrono dependency.
    let days = secs / 86400;
    let (h, m, sec) = ((secs % 86400) / 3600, (secs % 3600) / 60, secs % 60);
    let z = days + 719468;
    let era = z / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}{mo:02}{d:02}-{h:02}{m:02}{sec:02}")
}
pub struct StoreEntry {
    pub id: String,
    pub path: PathBuf,
    pub name: String,
    pub digest8: String,
    pub kit: bool,
    pub size: u64,
    pub modified: u64,
    exact: std::time::SystemTime,
    pub active: bool,
}
pub fn store_entries(data_dir: &Path) -> Res<Vec<StoreEntry>> {
    let dir = store_dir(data_dir);
    let mut out = vec![];
    let Ok(rd) = std::fs::read_dir(&dir) else {
        return Ok(out);
    };
    let stamp = installed_stamp(data_dir);
    for e in rd.filter_map(Result::ok) {
        let path = e.path();
        let id = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if !id.ends_with(".langolier") || id.starts_with("incoming-") {
            continue;
        }
        let stem = id.trim_end_matches(".langolier");
        let (stem, kit) = match stem.strip_suffix(".kit") {
            Some(s) => (s, true),
            None => (stem, false),
        };
        // <name>-<yyyymmdd>-<hhmmss>-<digest8>; the name itself may hold dashes.
        let mut parts = stem.rsplitn(4, '-');
        let digest8 = parts.next().unwrap_or("").to_string();
        let _time = parts.next();
        let _date = parts.next();
        let name = parts.next().unwrap_or(stem).to_string();
        let Ok(m) = path.metadata() else { continue };
        let exact = m.modified().unwrap_or(std::time::UNIX_EPOCH);
        let modified = exact
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // The display name lives inside the bundle; the file name only has the slug.
        let name = validate(&path).map(|a| a.name).unwrap_or(name);
        out.push(StoreEntry {
            active: stamp.starts_with(&digest8) && !digest8.is_empty(),
            id,
            path,
            name,
            digest8,
            kit,
            size: m.len(),
            modified,
            exact,
        });
    }
    out.sort_by_key(|e| std::cmp::Reverse(e.exact));
    Ok(out)
}
fn prune(data_dir: &Path) -> Res<()> {
    let entries = store_entries(data_dir)?;
    for e in entries.into_iter().skip(STORE_KEEP) {
        if !e.kit && !e.active {
            let _ = std::fs::remove_file(&e.path);
        }
    }
    Ok(())
}
fn store_entry(data_dir: &Path, id: &str) -> Res<StoreEntry> {
    store_entries(data_dir)?
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| "Unknown bundle.".to_string())
}
/// Makes a stored bundle the newest candidate and installs it now.
pub fn store_restore(data_dir: &Path, id: &str) -> Res<String> {
    let e = store_entry(data_dir, id)?;
    let bytes = std::fs::read(&e.path).map_err(err)?;
    touch(&e.path).map_err(err)?;
    install(&bytes, data_dir)?;
    Ok(e.name)
}
pub fn store_delete(data_dir: &Path, id: &str) -> Res<()> {
    let e = store_entry(data_dir, id)?;
    if e.kit {
        return Err("The bundle the kit shipped with cannot be deleted.".into());
    }
    if e.active {
        return Err("This bundle is the one running. Restore another one first.".into());
    }
    std::fs::remove_file(&e.path).map_err(err)
}
/// Import from the chat page: validate, archive, install.
pub fn store_import(data_dir: &Path, bytes: &[u8]) -> Res<String> {
    let staged = store_dir(data_dir).join(format!("incoming-{}.langolier", std::process::id()));
    std::fs::create_dir_all(store_dir(data_dir)).map_err(err)?;
    std::fs::write(&staged, bytes).map_err(err)?;
    let checked = validate(&staged);
    let _ = std::fs::remove_file(&staged);
    let a = checked?;
    let file = archive(data_dir, bytes, false)?;
    touch(&file).map_err(err)?;
    install(bytes, data_dir)?;
    Ok(a.name)
}
#[cfg(any(target_os = "macos", feature = "desktop"))]
pub fn base64_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf = 0u32;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => continue,
        } as u32;
        buf = (buf << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
            buf &= (1 << bits) - 1;
        }
    }
    out
}
/// Builds an .icns from the avatar, falling back to Langolier's icon.
#[cfg(target_os = "macos")]
fn write_icns(avatar: &str, dest: &Path) -> bool {
    if let Some((_, b64)) = avatar.split_once(",") {
        let png = base64_decode(b64);
        let work = std::env::temp_dir().join(format!("langolier-icon-{}", std::process::id()));
        let set = work.join("icon.iconset");
        let _ = std::fs::remove_dir_all(&work);
        if std::fs::create_dir_all(&set).is_ok() {
            let src = work.join("avatar.png");
            if std::fs::write(&src, &png).is_ok() {
                let mut ok = true;
                for (size, name) in [
                    (16, "icon_16x16"),
                    (32, "icon_16x16@2x"),
                    (32, "icon_32x32"),
                    (64, "icon_32x32@2x"),
                    (128, "icon_128x128"),
                    (256, "icon_128x128@2x"),
                    (256, "icon_256x256"),
                    (512, "icon_256x256@2x"),
                    (512, "icon_512x512"),
                    (1024, "icon_512x512@2x"),
                ] {
                    ok &= std::process::Command::new("sips")
                        .args([
                            "-s",
                            "format",
                            "png",
                            "-z",
                            &size.to_string(),
                            &size.to_string(),
                        ])
                        .arg(&src)
                        .arg("--out")
                        .arg(set.join(format!("{name}.png")))
                        .output()
                        .map(|o| o.status.success())
                        .unwrap_or(false);
                }
                if ok {
                    let made = std::process::Command::new("iconutil")
                        .args(["-c", "icns"])
                        .arg(&set)
                        .arg("-o")
                        .arg(dest)
                        .output()
                        .map(|o| o.status.success())
                        .unwrap_or(false);
                    let _ = std::fs::remove_dir_all(&work);
                    if made {
                        return true;
                    }
                }
            }
        }
        let _ = std::fs::remove_dir_all(&work);
    }
    if let Ok(dir) = exe_dir() {
        let own = dir.join("../Resources/icon.icns");
        if own.exists() {
            return std::fs::copy(own, dest).is_ok();
        }
    }
    false
}
#[cfg(target_os = "macos")]
fn write_app(dir: &Path, display: &str, mode: &str, slug: &str, avatar: &str) -> Res<PathBuf> {
    let app = dir.join(format!("{display}.app"));
    let _ = std::fs::remove_dir_all(&app);
    let macos = app.join("Contents/MacOS");
    let resources = app.join("Contents/Resources");
    std::fs::create_dir_all(&macos).map_err(err)?;
    std::fs::create_dir_all(&resources).map_err(err)?;
    let exe = std::env::current_exe().map_err(err)?;
    let target = macos.join(mode);
    std::fs::copy(&exe, &target).map_err(err)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755)).map_err(err)?;
    }
    let has_icon = write_icns(avatar, &resources.join("icon.icns"));
    let plist = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
  <key>CFBundleName</key><string>{display}</string>
  <key>CFBundleDisplayName</key><string>{display}</string>
  <key>CFBundleExecutable</key><string>{mode}</string>
  <key>CFBundleIdentifier</key><string>local.langolier.chatbot.{slug}.{mode}</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundleInfoDictionaryVersion</key><string>6.0</string>
  <key>LSMinimumSystemVersion</key><string>12.0</string>
  <key>NSHighResolutionCapable</key><true/>
  {icon}
</dict></plist>
"#,
        icon = if has_icon {
            "<key>CFBundleIconFile</key><string>icon</string>"
        } else {
            ""
        }
    );
    std::fs::write(app.join("Contents/Info.plist"), plist).map_err(err)?;
    Ok(app)
}
pub const LIGHT_CHAT_MODEL: &str = "qwen3:4b-instruct";
/// GGUF of an Ollama model, found in its blob store.
pub fn ollama_gguf(name: &str) -> Res<PathBuf> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|_| "HOME not set")?;
    let root = std::env::var("OLLAMA_MODELS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(home).join(".ollama/models"));
    let (repo, tag) = name.split_once(':').unwrap_or((name, "latest"));
    let repo = if repo.contains('/') {
        repo.to_string()
    } else {
        format!("library/{repo}")
    };
    let manifest = root
        .join("manifests/registry.ollama.ai")
        .join(&repo)
        .join(tag);
    let raw = std::fs::read_to_string(&manifest)
        .map_err(|_| format!("Ollama model \"{name}\" not found in {}", root.display()))?;
    let v: serde_json::Value = serde_json::from_str(&raw).map_err(err)?;
    let digest = v["layers"]
        .as_array()
        .and_then(|l| {
            l.iter()
                .find(|x| x["mediaType"] == "application/vnd.ollama.image.model")
        })
        .and_then(|x| x["digest"].as_str())
        .ok_or("Ollama manifest without a model layer")?;
    let blob = root.join("blobs").join(digest.replace(':', "-"));
    if !blob.is_file() {
        return Err(format!("Blob absent : {}", blob.display()));
    }
    Ok(blob)
}
/// Downloads the official embedding GGUF once into the app cache.
pub async fn cached_embed_gguf(
    db: &Db,
    tag: &str,
    progress: &(dyn Fn(&str, u64, u64) + Sync),
) -> Res<PathBuf> {
    let spec = crate::llm::embed_model(tag);
    cached_gguf(
        db,
        spec.gguf_url,
        spec.gguf_file,
        "Downloading the embedding model",
        100_000_000,
        progress,
    )
    .await
}
/// Chat GGUF of a curated tag, downloaded once into the app cache.
pub async fn cached_chat_gguf(
    db: &Db,
    tag: &str,
    progress: &(dyn Fn(&str, u64, u64) + Sync),
) -> Res<PathBuf> {
    let spec = crate::llm::chat_model(tag).ok_or_else(|| format!("No downloadable GGUF is known for {tag}. Pick one of the curated models or point to a GGUF file."))?;
    // Ollama already has the weights? Link them in instead of downloading again.
    let target = gguf_cache_dir(db).join(spec.gguf_file);
    if !target.is_file() {
        if let Ok(blob) = ollama_gguf(tag) {
            std::fs::create_dir_all(gguf_cache_dir(db)).map_err(err)?;
            if std::fs::hard_link(&blob, &target).is_err() {
                copy_with_progress(
                    &blob,
                    &target,
                    "Copying the chat model from Ollama",
                    progress,
                )?;
            }
            return Ok(target);
        }
    }
    cached_gguf(
        db,
        spec.gguf_url,
        spec.gguf_file,
        "Downloading the chat model",
        spec.bytes / 2,
        progress,
    )
    .await
}
pub fn gguf_cache_dir(db: &Db) -> PathBuf {
    db.root.join("gguf")
}
/// Downloads a GGUF into the app cache once; a partial file is redone.
async fn cached_gguf(
    db: &Db,
    url: &str,
    file: &str,
    label: &str,
    min_bytes: u64,
    progress: &(dyn Fn(&str, u64, u64) + Sync),
) -> Res<PathBuf> {
    let dir = gguf_cache_dir(db);
    std::fs::create_dir_all(&dir).map_err(err)?;
    let target = dir.join(file);
    if target.is_file()
        && std::fs::metadata(&target)
            .map(|m| m.len() > min_bytes)
            .unwrap_or(false)
    {
        return Ok(target);
    }
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(err)?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Downloading {file}: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Download of {file} refused: {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let tmp = dir.join(format!("{file}.part"));
    let mut out = std::fs::File::create(&tmp).map_err(err)?;
    let mut stream = resp.bytes_stream();
    let mut done = 0u64;
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(err)?;
        out.write_all(&chunk).map_err(err)?;
        done += chunk.len() as u64;
        progress(label, done, total);
    }
    drop(out);
    let mut head = [0u8; 4];
    let ok = std::fs::File::open(&tmp)
        .and_then(|mut f| f.read_exact(&mut head))
        .map(|_| &head == b"GGUF")
        .unwrap_or(false);
    if !ok {
        let _ = std::fs::remove_file(&tmp);
        return Err("The downloaded file is not a GGUF.".into());
    }
    std::fs::rename(&tmp, &target).map_err(err)?;
    Ok(target)
}
fn copy_with_progress(
    from: &Path,
    to: &Path,
    label: &str,
    progress: &(dyn Fn(&str, u64, u64) + Sync),
) -> Res<()> {
    use std::io::Read;
    let total = std::fs::metadata(from).map_err(err)?.len();
    let mut src = std::fs::File::open(from).map_err(err)?;
    let mut dst = std::fs::File::create(to).map_err(err)?;
    let mut buf = vec![0u8; 8 << 20];
    let mut done = 0u64;
    loop {
        let n = src.read(&mut buf).map_err(err)?;
        if n == 0 {
            break;
        }
        dst.write_all(&buf[..n]).map_err(err)?;
        done += n as u64;
        progress(label, done, total);
    }
    Ok(())
}
/// Recomputes every bundle vector with the embedded engine.
async fn reembed_bundle(
    langolier: &Path,
    embed_gguf: &Path,
    key: &str,
    progress: &(dyn Fn(&str, u64, u64) + Sync),
) -> Res<usize> {
    let rows: Vec<(i64, String, String, String)> = {
        let c = Connection::open(langolier).map_err(err)?;
        let mut st = c.prepare("SELECT ch.id,d.name,ch.locator,ch.text FROM chunks ch JOIN documents d ON d.id=ch.doc_id ORDER BY ch.id").map_err(err)?;
        let rows = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        rows
    };
    let total = rows.len();
    let model = embed_gguf.to_string_lossy().to_string();
    for (i, batch) in rows.chunks(32).enumerate() {
        let texts: Vec<String> = batch
            .iter()
            .map(|(_, n, l, t)| format!("Document: {n}\nSection: {l}\n{t}"))
            .collect();
        let vectors = crate::engine::embed(&model, &texts).await?;
        let mut c = Connection::open(langolier).map_err(err)?;
        let tx = c.transaction().map_err(err)?;
        for ((id, ..), v) in batch.iter().zip(vectors) {
            tx.execute(
                "UPDATE chunks SET embedding=?2,embedding_model=?3 WHERE id=?1",
                rusqlite::params![id, crate::rag::encode(&v), key],
            )
            .map_err(err)?;
        }
        tx.commit().map_err(err)?;
        progress(
            "Vectorisation des connaissances",
            ((i + 1) * 32).min(total) as u64,
            total as u64,
        );
    }
    Ok(total)
}
pub async fn write_kit(
    db: &Db,
    a: &Assistant,
    dest: &Path,
    engine: &str,
    progress: &(dyn Fn(&str, u64, u64) + Sync),
) -> Res<serde_json::Value> {
    let slug = slug(&a.name);
    let dir = dest.join(&slug);
    std::fs::create_dir_all(&dir).map_err(err)?;
    let langolier = dir.join(format!("{slug}.langolier"));
    let base = db.settings()?;
    let effective = assistant::effective(&base, Some(a))?;
    let mut forced: Option<crate::db::Settings> = None;
    let mut models: Vec<serde_json::Value> = vec![];
    if engine == "embedded" || engine == "embedded-light" {
        let models_dir = dir.join("models");
        std::fs::create_dir_all(&models_dir).map_err(err)?;
        let mut s = effective.clone();
        if engine == "embedded-light" {
            s.provider = crate::llm::EMBEDDED.into();
            s.model = LIGHT_CHAT_MODEL.into();
        }
        if !crate::llm::is_cloud(&s.provider) {
            // Ollama's blob when it has one, else the curated GGUF from the cache.
            let src = match ollama_gguf(&s.model) {
                Ok(p) if s.provider != crate::llm::EMBEDDED => p,
                _ => {
                    let resolved = crate::engine::resolve(&s.model)?;
                    if resolved.is_file() {
                        resolved
                    } else {
                        cached_chat_gguf(db, &s.model, progress).await?
                    }
                }
            };
            let dst = models_dir.join("chat.gguf");
            copy_with_progress(&src, &dst, "Copying the chat model", progress)?;
            models.push(serde_json::json!({"role": "chat", "from": s.model, "file": "models/chat.gguf", "bytes": std::fs::metadata(&dst).map(|m| m.len()).unwrap_or(0)}));
            s.provider = crate::llm::EMBEDDED.into();
            s.model = "models/chat.gguf".into();
            s.api_key.clear();
        }
        // The kit embedder is the profile's, as an official GGUF.
        let embed_spec = crate::llm::embed_model(&s.embedding_model);
        let embed_src = if s.embedding_endpoint.trim() == crate::llm::EMBEDDED {
            crate::engine::resolve(&s.embedding_model)?
        } else {
            cached_embed_gguf(db, &s.embedding_model, progress).await?
        };
        let embed_dst = models_dir.join("embed.gguf");
        copy_with_progress(
            &embed_src,
            &embed_dst,
            "Copying the embedding model",
            progress,
        )?;
        models.push(serde_json::json!({"role": "embed", "from": embed_spec.gguf_file, "file": "models/embed.gguf", "bytes": std::fs::metadata(&embed_dst).map(|m| m.len()).unwrap_or(0)}));
        s.embedding_endpoint = crate::llm::EMBEDDED.into();
        s.embedding_model = "models/embed.gguf".into();
        forced = Some(s);
    }
    let payload = write_langolier_with(db, a, &langolier, forced.as_ref())?;
    let mut reembedded = 0;
    if let Some(s) = &forced {
        let key = crate::rag::embedding_key(s);
        reembedded =
            reembed_bundle(&langolier, &dir.join("models/embed.gguf"), &key, progress).await?;
        let c = Connection::open(&langolier).map_err(err)?;
        c.execute_batch("VACUUM;").map_err(err)?;
    }
    let mut launchers: Vec<String> = vec![];
    #[cfg(target_os = "macos")]
    {
        write_app(&dir, &a.name, "chatbotgui", &slug, &a.avatar)?;
        write_app(
            &dir,
            &format!("{} Web", a.name),
            "chatbotweb",
            &slug,
            &a.avatar,
        )?;
        launchers.push(format!("{}.app", a.name));
        launchers.push(format!("{} Web.app", a.name));
    }
    #[cfg(not(target_os = "macos"))]
    {
        let exe = std::env::current_exe().map_err(err)?;
        let ext = if cfg!(target_os = "windows") {
            ".exe"
        } else {
            ""
        };
        for mode in ["chatbotgui", "chatbotweb"] {
            let target = dir.join(format!("{mode}{ext}"));
            std::fs::copy(&exe, &target).map_err(err)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o755))
                    .map_err(err)?;
            }
            launchers.push(format!("{mode}{ext}"));
        }
    }
    // Launchers for other systems, built by CI and dropped in by the user.
    let runners = db.root.join("runners");
    let mut foreign = 0;
    if runners.is_dir() {
        let out = dir.join("other-platform-launchers");
        for e in std::fs::read_dir(&runners)
            .map_err(err)?
            .filter_map(Result::ok)
        {
            if e.path().is_file() {
                std::fs::create_dir_all(&out).map_err(err)?;
                std::fs::copy(e.path(), out.join(e.file_name())).map_err(err)?;
                foreign += 1;
            }
        }
    }
    let requirements = if forced.is_some() {
        "No prerequisites: the llama.cpp engine lives in the launchers and the models in `models/`."
            .to_string()
    } else if crate::llm::is_cloud(&effective.provider) {
        format!("Cloud provider `{}` (model `{}`): the API key is embedded. For meaning-based search, Ollama with `{}` is used when present; otherwise search stays lexical.", effective.provider, effective.model, effective.embedding_model)
    } else {
        format!("Ollama must run on the machine with:\n\n- `{}` (chat): `ollama pull {}`\n- `{}` (embeddings, required for meaning-based search): `ollama pull {}`\n\nMissing models are pulled automatically on first start if Ollama answers on `{}`.", effective.model, effective.model, effective.embedding_model, effective.embedding_model, effective.endpoint)
    };
    write_kit_scripts(&dir, &a.name, &slug, foreign, &requirements)?;
    Ok(
        serde_json::json!({"path": dir, "payload": payload, "launchers": launchers, "foreign_runners": foreign, "models": models, "reembedded": reembedded, "engine": engine}),
    )
}
fn write_kit_scripts(
    dir: &Path,
    name: &str,
    slug: &str,
    foreign: usize,
    requirements: &str,
) -> Res<()> {
    let web_bin = if cfg!(target_os = "macos") {
        format!("\"./{name} Web.app/Contents/MacOS/chatbotweb\"")
    } else {
        "./chatbotweb".to_string()
    };
    let sh = format!("#!/bin/bash\n# Headless server: chat page on the network, port 8080 (PORT variable).\ncd \"$(dirname \"$0\")\"\n{web_bin} --serve --host 0.0.0.0 --port \"${{PORT:-8080}}\"\n");
    let bat = "@echo off\r\nrem Headless server: chat page on the network, port 8080.\r\ncd /d \"%~dp0\"\r\nchatbotweb.exe --serve --host 0.0.0.0 --port 8080\r\n".to_string();
    let readme = format!(
        "# {name}\n\nChatbot exported from Langolier.\n\n## Contents\n\n- `{slug}.langolier`: the profile, its settings and its knowledge. **This is the only file to replace when updating** (name, mission, image, theme, knowledge). Drop the new version here and the launcher picks it up in under 30 seconds, without a restart, keeping conversations.\n- `{name}.app` (Mac) or `chatbotgui`: standalone chat window.\n- `{name} Web.app` (Mac) or `chatbotweb`: opens the chat page in the browser (port 8787).\n- `server.sh` / `server.bat`: the same page served on the network, headless, port 8080.\n{foreign_note}\n## Other systems\n\nLaunchers are platform specific; the `.langolier` is universal. For Windows or Linux, grab that platform's launchers (`chatbotgui`, `chatbotweb`) from the Langolier releases and drop them next to the `.langolier`.\n\n## Engine and prerequisites\n\n{requirements}\n\nOptions: `--serve --host 0.0.0.0 --port 8080 --data /path`, `--gui`.\n",
        foreign_note = if foreign > 0 { format!("- `other-platform-launchers/`: {foreign} launcher(s) for other systems.\n") } else { String::new() }
    );
    #[cfg_attr(not(unix), allow(unused_variables))]
    for (file, content, exec) in [
        ("server.sh", sh, true),
        ("server.bat", bat, false),
        ("README.md", readme, false),
    ] {
        let p = dir.join(file);
        let mut f = std::fs::File::create(&p).map_err(err)?;
        f.write_all(content.as_bytes()).map_err(err)?;
        #[cfg(unix)]
        if exec {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).map_err(err)?;
        }
    }
    Ok(())
}
pub fn exe_dir() -> Res<PathBuf> {
    let exe = std::env::current_exe().map_err(err)?;
    Ok(exe
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from(".")))
}
/// Where to look for .langolier files.
pub fn bundle_dir() -> Res<PathBuf> {
    // --bundle <dir> or LANGOLIER_BUNDLE_DIR.
    if let Ok(d) = std::env::var("LANGOLIER_BUNDLE_DIR") {
        if !d.trim().is_empty() {
            return Ok(PathBuf::from(d.trim()));
        }
    }
    let dir = exe_dir()?;
    if dir.ends_with("Contents/MacOS") {
        if let Some(outside) = dir.parent().and_then(Path::parent).and_then(Path::parent) {
            return Ok(outside.to_path_buf());
        }
    }
    Ok(dir)
}
pub fn default_data_dir(slug: &str) -> Res<PathBuf> {
    let local = bundle_dir()?.join(format!("{slug}.data"));
    if std::fs::create_dir_all(&local).is_ok()
        && std::fs::metadata(&local)
            .map(|m| !m.permissions().readonly())
            .unwrap_or(false)
    {
        let probe = local.join(".write-test");
        if std::fs::write(&probe, b"ok").is_ok() {
            let _ = std::fs::remove_file(&probe);
            return Ok(local);
        }
    }
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".into());
    let base = if cfg!(target_os = "macos") {
        PathBuf::from(home).join("Library/Application Support/Langolier Chatbots")
    } else if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(home))
            .join("Langolier Chatbots")
    } else {
        std::env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(home).join(".local/share"))
            .join("langolier-chatbots")
    };
    Ok(base.join(slug))
}
/// A bundle's profile slug, used to name the data folder.
pub fn slug_of(path: &Path) -> String {
    validate(path)
        .map(|a| slug(&a.name))
        .unwrap_or_else(|_| "chatbot".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn store_archives_restores_and_protects_the_kit() {
        let root = tempfile::tempdir().unwrap();
        let kit = root.path().join("kit");
        let data = root.path().join("state");
        std::fs::create_dir_all(&kit).unwrap();
        let src = Db::new(&root.path().join("src")).unwrap();
        let mut bundles = vec![];
        for name in ["Alpha", "Beta"] {
            let a = assistant::save(
                &src,
                Assistant {
                    name: name.into(),
                    ..Default::default()
                },
            )
            .unwrap();
            let path = root.path().join(format!("{name}.langolier"));
            write_langolier(&src, &a, &path).unwrap();
            bundles.push(std::fs::read(&path).unwrap());
        }
        // First start: the kit's bundle is installed and archived with the .kit marker.
        std::fs::write(kit.join("alpha.langolier"), &bundles[0]).unwrap();
        assert_eq!(
            sync(&kit, &data).unwrap().as_deref(),
            Some("alpha.langolier")
        );
        let entries = store_entries(&data).unwrap();
        assert_eq!(entries.len(), 1);
        assert!(entries[0].kit && entries[0].active);
        assert_eq!(entries[0].name, "Alpha");
        // Import from the page: archived, installed, newest, no reversal by sync.
        assert_eq!(store_import(&data, &bundles[1]).unwrap(), "Beta");
        assert!(sync(&kit, &data).unwrap().is_none());
        let entries = store_entries(&data).unwrap();
        assert_eq!(entries.len(), 2);
        assert!(entries[0].active && entries[0].name == "Beta");
        // Same bytes twice never make two entries.
        store_import(&data, &bundles[1]).unwrap();
        assert_eq!(store_entries(&data).unwrap().len(), 2);
        // The kit entry and the running one refuse deletion.
        let kit_id = entries[1].id.clone();
        let beta_id = entries[0].id.clone();
        assert!(store_delete(&data, &kit_id).unwrap_err().contains("kit"));
        assert!(store_delete(&data, &beta_id)
            .unwrap_err()
            .contains("running"));
        // Restore the kit, then Beta becomes deletable.
        assert_eq!(store_restore(&data, &kit_id).unwrap(), "Alpha");
        assert!(sync(&kit, &data).unwrap().is_none());
        assert!(store_entries(&data)
            .unwrap()
            .iter()
            .any(|e| e.id == kit_id && e.active));
        store_delete(&data, &beta_id).unwrap();
        assert_eq!(store_entries(&data).unwrap().len(), 1);
        assert!(store_delete(&data, "../langolier.sqlite3").is_err());
    }
    #[test]
    fn export_scope_and_update_keeps_conversations() {
        let root = tempfile::tempdir().unwrap();
        let db = Db::new(root.path()).unwrap();
        let c = db.conn().unwrap();
        for (id, name) in [("a", "Petite enfance"), ("b", "Accounting")] {
            c.execute("INSERT INTO documents(id,name,source,kind,status,created,updated)VALUES(?1,?2,?1,'md','ready',1,1)", rusqlite::params![id, name]).unwrap();
            c.execute(
                "INSERT INTO chunks(doc_id,ordinal,text,locator)VALUES(?1,0,?2,'p1')",
                rusqlite::params![id, format!("texte {name}")],
            )
            .unwrap();
        }
        c.execute(
            "INSERT INTO conversations(id,title,created)VALUES('conv','x',1)",
            [],
        )
        .unwrap();
        drop(c);
        if let Ok(keep) = std::env::var("LANGOLIER_KEEP_BUNDLE") {
            let a0 = assistant::save(
                &db,
                Assistant {
                    name: "Nounou".into(),
                    mission: "Tu aides les parents de jeunes enfants, chaleureusement.".into(),
                    welcome: "Bonjour, posez-moi vos questions.".into(),
                    theme: "sauge".into(),
                    scope: json!({"documents": ["a"]}),
                    ..Default::default()
                },
            )
            .unwrap();
            write_langolier(&db, &a0, Path::new(&keep)).unwrap();
            assistant::remove(&db, &a0.id).unwrap();
        }
        let mut s = db.settings().unwrap();
        s.provider = "anthropic".into();
        s.endpoint = "https://api.anthropic.com".into();
        s.api_key = "sk-ant-test".into();
        s.model = "claude-opus-5".into();
        db.set_settings(&s).unwrap();
        let a = assistant::save(
            &db,
            Assistant {
                name: "Nounou".into(),
                mission: "Answer parents.".into(),
                scope: json!({"documents": ["a"]}),
                overrides: json!({"temperature": 0.7}),
                ..Default::default()
            },
        )
        .unwrap();
        let file = root.path().join("nounou.langolier");
        write_langolier(&db, &a, &file).unwrap();
        assert_eq!(validate(&file).unwrap().name, "Nounou");
        let bc = Connection::open(&file).unwrap();
        let docs: i64 = bc
            .query_row("SELECT count(*) FROM documents", [], |r| r.get(0))
            .unwrap();
        let convs: i64 = bc
            .query_row("SELECT count(*) FROM conversations", [], |r| r.get(0))
            .unwrap();
        let raw: String = bc
            .query_row("SELECT value FROM settings", [], |r| r.get(0))
            .unwrap();
        let exported: crate::db::Settings = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            (docs, convs),
            (1, 0),
            "perimetre applique, conversations exclues"
        );
        assert_eq!(exported.active_assistant, a.id);
        assert_eq!(exported.api_key, "sk-ant-test", "cle cloud embarquee");
        assert!(
            (exported.temperature - 0.7).abs() < 1e-6,
            "surcharge fusionnee"
        );
        drop(bc);
        let data = tempfile::tempdir().unwrap();
        let bytes = std::fs::read(&file).unwrap();
        install(&bytes, data.path()).unwrap();
        let live = Db::new(data.path()).unwrap();
        let conv = crate::pipeline::new_conversation(&live, Some(&a.id)).unwrap();
        live.conn().unwrap().execute("INSERT INTO messages(id,conversation_id,role,content,created)VALUES('m1',?1,'user','salut',1)", [&conv]).unwrap();
        let mut a2 = a.clone();
        a2.welcome = "Bonjour !".into();
        assistant::save(&db, a2).unwrap();
        let file2 = root.path().join("nounou-v2.langolier");
        write_langolier(&db, &assistant::get(&db, &a.id).unwrap().unwrap(), &file2).unwrap();
        let bytes2 = std::fs::read(&file2).unwrap();
        assert_ne!(digest(&bytes), digest(&bytes2));
        install(&bytes2, data.path()).unwrap();
        let live = Db::new(data.path()).unwrap();
        let lc = live.conn().unwrap();
        let msgs: i64 = lc
            .query_row(
                "SELECT count(*) FROM messages WHERE conversation_id=?1",
                [&conv],
                |r| r.get(0),
            )
            .unwrap();
        let welcome: String = lc
            .query_row("SELECT welcome FROM assistants", [], |r| r.get(0))
            .unwrap();
        assert_eq!((msgs, welcome.as_str()), (1, "Bonjour !"));
        assert_eq!(installed_stamp(data.path()), digest(&bytes2));
        assert_eq!(slug("Ma Nounou 2025 !"), "ma-nounou-2025");
    }
}

#[cfg(test)]
mod kit_tests {
    use super::*;
    use serde_json::json;
    #[test]
    #[ignore = "Copie plusieurs Go ; cargo test kit_tests -- --ignored --nocapture"]
    fn embedded_kit() {
        let out = PathBuf::from(std::env::var("LANGOLIER_KIT_OUT").expect("LANGOLIER_KIT_OUT"));
        let root = tempfile::tempdir().unwrap();
        let db = Db::new(root.path()).unwrap();
        if let Ok(src) = std::env::var("LANGOLIER_GGUF_EMBED") {
            std::fs::create_dir_all(root.path().join("gguf")).unwrap();
            std::fs::copy(
                src,
                root.path()
                    .join("gguf")
                    .join(crate::llm::embed_model("embeddinggemma").gguf_file),
            )
            .unwrap();
        }
        let c = db.conn().unwrap();
        c.execute("INSERT INTO documents(id,name,source,kind,status,created,updated)VALUES('a','Guide des parents','a','md','ready',1,1)", []).unwrap();
        for (i, t) in [
            "Le sommeil du nourrisson : entre 0 et 3 mois, un bébé dort 14 à 17 heures par jour, par cycles courts.",
            "La diversification alimentaire commence entre 4 et 6 mois, un aliment nouveau à la fois, jamais de miel avant un an.",
            "La fièvre chez le bébé de moins de 3 mois impose une consultation le jour même.",
        ].iter().enumerate() {
            c.execute("INSERT INTO chunks(doc_id,ordinal,text,locator)VALUES('a',?1,?2,?3)", rusqlite::params![i as i64, t, format!("§{}", i + 1)]).unwrap();
        }
        drop(c);
        let a = assistant::save(
            &db,
            Assistant {
                name: "Nounou".into(),
                mission: "Tu aides les parents de jeunes enfants avec bienveillance et prudence."
                    .into(),
                welcome: "Bonjour, je réponds à partir du guide des parents.".into(),
                theme: "sauge".into(),
                ..Default::default()
            },
        )
        .unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let progress = |stage: &str, done: u64, total: u64| {
            if total > 0 && (done == total || done % (64 << 20) < (8 << 20)) {
                eprintln!("  {stage} {}/{}", done >> 20, total >> 20);
            }
        };
        let r = rt
            .block_on(write_kit(&db, &a, &out, "embedded", &progress))
            .unwrap();
        println!("KIT {r}");
        let dir = out.join("nounou");
        assert!(dir.join("models/chat.gguf").is_file());
        assert!(dir.join("models/embed.gguf").is_file());
        let bc = Connection::open(dir.join("nounou.langolier")).unwrap();
        let raw: String = bc
            .query_row("SELECT value FROM settings", [], |r| r.get(0))
            .unwrap();
        let s: crate::db::Settings = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            (
                s.provider.as_str(),
                s.model.as_str(),
                s.embedding_endpoint.as_str(),
                s.embedding_model.as_str()
            ),
            (
                "embedded",
                "models/chat.gguf",
                "embedded",
                "models/embed.gguf"
            )
        );
        let (n, keyed): (i64, i64) = bc
            .query_row(
                "SELECT count(*),sum(embedding IS NOT NULL AND embedding_model=?1) FROM chunks",
                [crate::rag::embedding_key(&s)],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(
            (n, keyed),
            (3, 3),
            "every passage re-vectorised with the embedded key"
        );
        assert_eq!(r["reembedded"], json!(3));
        crate::engine::shutdown();
    }
}

#[cfg(test)]
mod export_tests {
    use super::*;
    #[test]
    #[ignore = "utilise une base reelle ; cargo test export_tests -- --ignored --nocapture"]
    fn export_named_profile() {
        let root = PathBuf::from(std::env::var("LANGOLIER_DB").expect("LANGOLIER_DB"));
        let name = std::env::var("LANGOLIER_ASSISTANT").expect("LANGOLIER_ASSISTANT");
        let out = PathBuf::from(std::env::var("LANGOLIER_KIT_OUT").expect("LANGOLIER_KIT_OUT"));
        let engine = std::env::var("LANGOLIER_ENGINE").unwrap_or_else(|_| "profile".into());
        let db = Db::new(&root).unwrap();
        let a = assistant::list(&db)
            .unwrap()
            .into_iter()
            .find(|a| a.name == name)
            .expect("profile not found");
        let rt = tokio::runtime::Runtime::new().unwrap();
        let progress = |stage: &str, done: u64, total: u64| eprintln!("  {stage} {done}/{total}");
        let r = rt
            .block_on(write_kit(&db, &a, &out, &engine, &progress))
            .unwrap();
        println!("KIT {r}");
        crate::engine::shutdown();
    }
}
