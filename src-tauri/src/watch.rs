//! Watched folders, polled on a timer.
use crate::{
    db::{err, now, Db, Res},
    ingest,
};
use rusqlite::params;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const MODES: [&str; 2] = ["sync", "hoover"];
/// A file touched more recently than this may still be copying.
const SETTLE_SECS: i64 = 3;

pub fn schema() -> &'static str {
    "CREATE TABLE IF NOT EXISTS watches(id TEXT PRIMARY KEY,path TEXT NOT NULL UNIQUE,mode TEXT NOT NULL,enabled INTEGER NOT NULL DEFAULT 1,recursive INTEGER NOT NULL DEFAULT 1,created INTEGER NOT NULL,last_scan INTEGER,last_error TEXT);
     CREATE TABLE IF NOT EXISTS watch_files(watch_id TEXT NOT NULL REFERENCES watches(id) ON DELETE CASCADE,path TEXT NOT NULL,size INTEGER NOT NULL,mtime INTEGER NOT NULL,doc_id TEXT,PRIMARY KEY(watch_id,path));"
}
pub fn migrate(c: &rusqlite::Connection) -> Res<()> {
    let has: i64 = c
        .query_row(
            "SELECT count(*) FROM pragma_table_info('documents') WHERE name='watch_id'",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    if has == 0 {
        c.execute_batch("ALTER TABLE documents ADD COLUMN watch_id TEXT; CREATE INDEX IF NOT EXISTS docs_watch ON documents(watch_id);")
            .map_err(err)?;
    }
    let has: i64 = c
        .query_row(
            "SELECT count(*) FROM pragma_table_info('watches') WHERE name='interval'",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    if has == 0 {
        // Watch interval in seconds; NULL means the global setting.
        c.execute_batch("ALTER TABLE watches ADD COLUMN interval INTEGER;")
            .map_err(err)?;
    }
    Ok(())
}
fn check_interval(secs: Option<i64>) -> Res<()> {
    match secs {
        Some(v) if !(10..=31 * 86400).contains(&v) => {
            Err("Intervalle entre 10 secondes et 31 jours.".into())
        }
        _ => Ok(()),
    }
}
fn validate_dir(raw: &str, db: &Db) -> Res<PathBuf> {
    let p = PathBuf::from(raw.trim());
    if !p.is_absolute() {
        return Err("Indiquez un chemin absolu.".into());
    }
    let p = p
        .canonicalize()
        .map_err(|e| format!("Dossier inaccessible : {e}"))?;
    if !p.is_dir() {
        return Err("This path is not a folder.".into());
    }
    let root = db.root.canonicalize().unwrap_or(db.root.clone());
    if p.starts_with(&root) || root.starts_with(&p) {
        return Err("Langolier's own data folder cannot be watched.".into());
    }
    Ok(p)
}
pub fn add(db: &Db, path: &str, mode: &str, recursive: bool, interval: Option<i64>) -> Res<Value> {
    if !MODES.contains(&mode) {
        return Err("Mode de vigie inconnu".into());
    }
    check_interval(interval)?;
    let p = validate_dir(path, db)?;
    let id = uuid::Uuid::new_v4().to_string();
    db.conn()?
        .execute(
            "INSERT INTO watches(id,path,mode,enabled,recursive,created,interval) VALUES(?1,?2,?3,1,?4,?5,?6)",
            params![id, p.to_string_lossy(), mode, recursive as i64, now(), interval],
        )
        .map_err(|e| {
            if e.to_string().contains("UNIQUE") {
                "This folder is already watched.".to_string()
            } else {
                e.to_string()
            }
        })?;
    Ok(json!({"id": id}))
}
pub fn update(
    db: &Db,
    id: &str,
    enabled: Option<bool>,
    mode: Option<&str>,
    recursive: Option<bool>,
    interval: Option<Option<i64>>,
) -> Res<()> {
    let c = db.conn()?;
    if let Some(i) = interval {
        check_interval(i)?;
        // An interval of 0 puts the watch back on the global setting.
        c.execute(
            "UPDATE watches SET interval=?2 WHERE id=?1",
            params![id, i.filter(|v| *v > 0)],
        )
        .map_err(err)?;
    }
    if let Some(e) = enabled {
        c.execute(
            "UPDATE watches SET enabled=?2,last_error=NULL WHERE id=?1",
            params![id, e as i64],
        )
        .map_err(err)?;
    }
    if let Some(m) = mode {
        if !MODES.contains(&m) {
            return Err("Mode de vigie inconnu".into());
        }
        c.execute("UPDATE watches SET mode=?2 WHERE id=?1", params![id, m])
            .map_err(err)?;
    }
    if let Some(r) = recursive {
        c.execute(
            "UPDATE watches SET recursive=?2 WHERE id=?1",
            params![id, r as i64],
        )
        .map_err(err)?;
    }
    Ok(())
}
/// Drops the watch.
pub fn remove(db: &Db, id: &str, forget: bool) -> Res<Value> {
    let mut c = db.conn()?;
    let tx = c.transaction().map_err(err)?;
    let docs = if forget {
        tx.execute(
            "DELETE FROM documents WHERE watch_id=?1 AND status!='processing'",
            [id],
        )
        .map_err(err)?
    } else {
        tx.execute("UPDATE documents SET watch_id=NULL WHERE watch_id=?1", [id])
            .map_err(err)?
    };
    let n = tx
        .execute("DELETE FROM watches WHERE id=?1", [id])
        .map_err(err)?;
    tx.commit().map_err(err)?;
    if n == 0 {
        return Err("Watch not found".into());
    }
    Ok(json!({"documents": docs, "forgotten": forget}))
}
fn list_files(root: &Path, recursive: bool) -> Vec<PathBuf> {
    let walker = walkdir::WalkDir::new(root)
        .follow_links(false)
        .max_depth(if recursive { usize::MAX } else { 1 });
    walker
        .into_iter()
        .filter_entry(|e| {
            // The root itself may be hidden; the filter only applies to its contents.
            e.depth() == 0
                || !e.file_name().to_str().is_some_and(|n| {
                    n.starts_with('.')
                        || matches!(n, "node_modules" | "target" | "venv" | "__pycache__")
                })
        })
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            let ext = p
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            ingest::TYPES.contains(&ext.as_str())
        })
        .collect()
}
fn unique_destination(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    let (stem, ext) = match name.rsplit_once('.') {
        Some((s, e)) => (s.to_string(), format!(".{e}")),
        None => (name.to_string(), String::new()),
    };
    (2..)
        .map(|i| dir.join(format!("{stem} ({i}){ext}")))
        .find(|p| !p.exists())
        .unwrap()
}
fn move_file(from: &Path, to: &Path) -> Res<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(err)?;
    }
    if std::fs::rename(from, to).is_ok() {
        return Ok(());
    }
    std::fs::copy(from, to).map_err(err)?;
    std::fs::remove_file(from).map_err(err)?;
    Ok(())
}
fn file_stamp(p: &Path) -> Res<(i64, i64)> {
    let m = p.metadata().map_err(err)?;
    let mtime = m
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Ok((m.len() as i64, mtime))
}
fn kind_of(p: &Path) -> String {
    p.extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase()
}
pub fn scan(db: &Db, id: &str) -> Res<Value> {
    let c = db.conn()?;
    let (path, mode, recursive): (String, String, bool) = c
        .query_row(
            "SELECT path,mode,recursive FROM watches WHERE id=?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get::<_, i64>(2)? != 0)),
        )
        .map_err(|_| "Watch not found".to_string())?;
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        let msg = "Folder not found (volume unmounted?)";
        c.execute(
            "UPDATE watches SET last_error=?2,last_scan=?3 WHERE id=?1",
            params![id, msg, now()],
        )
        .map_err(err)?;
        return Err(msg.into());
    }
    let (mut added, mut updated, mut moved, mut errors) = (0, 0, 0, vec![]);
    let t = now();
    let mut seen = std::collections::HashSet::new();
    for file in list_files(&root, recursive) {
        let Ok((size, mtime)) = file_stamp(&file) else {
            continue;
        };
        if t - mtime < SETTLE_SECS {
            continue; // still being written
        }
        let source = file.to_string_lossy().to_string();
        seen.insert(source.clone());
        if mode == "hoover" {
            let dest_dir = db.root.join("imports").join(id);
            let dest = unique_destination(
                &dest_dir,
                &file.file_name().unwrap_or_default().to_string_lossy(),
            );
            if let Err(e) = move_file(&file, &dest) {
                errors.push(format!("{} : {e}", file.display()));
                continue;
            }
            let dest_s = dest.to_string_lossy().to_string();
            let doc_id = uuid::Uuid::new_v4().to_string();
            c.execute("INSERT INTO documents(id,name,source,kind,bytes,created,updated,watch_id)VALUES(?1,?2,?3,?4,?5,?6,?6,?7)",
                params![doc_id, dest.file_name().unwrap_or_default().to_string_lossy(), dest_s, kind_of(&dest), size, now(), id]).map_err(err)?;
            c.execute("INSERT OR REPLACE INTO watch_files(watch_id,path,size,mtime,doc_id) VALUES(?1,?2,?3,?4,?5)", params![id, dest_s, size, mtime, doc_id]).map_err(err)?;
            moved += 1;
            continue;
        }
        let known: Option<(i64, i64, Option<String>)> = c
            .query_row(
                "SELECT size,mtime,doc_id FROM watch_files WHERE watch_id=?1 AND path=?2",
                params![id, source],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .ok();
        match known {
            Some((s0, m0, Some(doc_id))) if s0 == size && m0 == mtime => {
                // Unchanged, but the document may have been removed by hand.
                let alive: i64 = c
                    .query_row(
                        "SELECT count(*) FROM documents WHERE id=?1",
                        [&doc_id],
                        |r| r.get(0),
                    )
                    .map_err(err)?;
                if alive == 0 {
                    c.execute(
                        "DELETE FROM watch_files WHERE watch_id=?1 AND path=?2",
                        params![id, source],
                    )
                    .map_err(err)?;
                }
            }
            Some((_, _, Some(doc_id))) => {
                c.execute("UPDATE documents SET status='queued',stage='Changed in the watch',error=NULL,bytes=?2,updated=?3 WHERE id=?1 AND status!='processing'", params![doc_id, size, now()]).map_err(err)?;
                c.execute(
                    "UPDATE watch_files SET size=?3,mtime=?4 WHERE watch_id=?1 AND path=?2",
                    params![id, source, size, mtime],
                )
                .map_err(err)?;
                updated += 1;
            }
            _ => {
                // New to the watch; an earlier manual import is adopted.
                let existing: Option<String> = c
                    .query_row("SELECT id FROM documents WHERE source=?1", [&source], |r| {
                        r.get(0)
                    })
                    .ok();
                let doc_id = match existing {
                    Some(d) => {
                        c.execute(
                            "UPDATE documents SET watch_id=?2 WHERE id=?1",
                            params![d, id],
                        )
                        .map_err(err)?;
                        d
                    }
                    None => {
                        let d = uuid::Uuid::new_v4().to_string();
                        c.execute("INSERT INTO documents(id,name,source,kind,bytes,created,updated,watch_id)VALUES(?1,?2,?3,?4,?5,?6,?6,?7)",
                            params![d, file.file_name().unwrap_or_default().to_string_lossy(), source, kind_of(&file), size, now(), id]).map_err(err)?;
                        added += 1;
                        d
                    }
                };
                c.execute("INSERT OR REPLACE INTO watch_files(watch_id,path,size,mtime,doc_id) VALUES(?1,?2,?3,?4,?5)", params![id, source, size, mtime, doc_id]).map_err(err)?;
            }
        }
    }
    if mode == "sync" {
        // Files gone from the folder.
        let mut st = c
            .prepare("SELECT path FROM watch_files WHERE watch_id=?1")
            .map_err(err)?;
        let gone: Vec<String> = st
            .query_map([id], |r| r.get::<_, String>(0))
            .map_err(err)?
            .filter_map(Result::ok)
            .filter(|p| !seen.contains(p) && !Path::new(p).exists())
            .collect();
        drop(st);
        for p in &gone {
            c.execute(
                "DELETE FROM watch_files WHERE watch_id=?1 AND path=?2",
                params![id, p],
            )
            .map_err(err)?;
        }
    }
    let last_error = (!errors.is_empty()).then(|| {
        errors
            .iter()
            .take(3)
            .cloned()
            .collect::<Vec<_>>()
            .join(" · ")
    });
    c.execute(
        "UPDATE watches SET last_scan=?2,last_error=?3 WHERE id=?1",
        params![id, now(), last_error],
    )
    .map_err(err)?;
    Ok(json!({"added": added, "updated": updated, "moved": moved, "errors": errors}))
}
/// Scan loop.
pub async fn worker(db: Db, notify: impl Fn() + Send + 'static) {
    loop {
        let interval = db.settings().map(|s| s.watch_interval.max(5)).unwrap_or(30) as i64;
        let due: Vec<String> = db
            .conn()
            .and_then(|c| {
                let mut st = c
                    .prepare("SELECT id FROM watches WHERE enabled=1 AND (last_scan IS NULL OR last_scan<=?1-coalesce(interval,?2))")
                    .map_err(err)?;
                let v = st
                    .query_map(params![now(), interval], |r| r.get::<_, String>(0))
                    .map_err(err)?
                    .filter_map(Result::ok)
                    .collect();
                Ok(v)
            })
            .unwrap_or_default();
        let mut changed = false;
        for id in due {
            let db2 = db.clone();
            let id2 = id.clone();
            let result = tokio::task::spawn_blocking(move || scan(&db2, &id2)).await;
            match result {
                Ok(Ok(v)) => {
                    changed |= v["added"].as_u64().unwrap_or(0)
                        + v["updated"].as_u64().unwrap_or(0)
                        + v["moved"].as_u64().unwrap_or(0)
                        > 0;
                }
                Ok(Err(e)) => {
                    let _ = db.conn().and_then(|c| {
                        c.execute(
                            "UPDATE watches SET last_error=?2,last_scan=?3 WHERE id=?1",
                            params![id, e, now()],
                        )
                        .map_err(err)
                    });
                    changed = true;
                }
                Err(_) => {}
            }
        }
        if changed {
            notify();
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settle() {
        std::thread::sleep(Duration::from_millis(3300));
    }
    #[test]
    fn sync_then_hoover() {
        let data = tempfile::tempdir().unwrap();
        let folder = tempfile::tempdir().unwrap();
        let db = Db::new(data.path()).unwrap();
        std::fs::write(folder.path().join("note.md"), "bonjour").unwrap();
        std::fs::write(folder.path().join("ignore.xyz"), "x").unwrap();
        let id = add(&db, folder.path().to_str().unwrap(), "sync", true, Some(60)).unwrap()["id"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            add(&db, folder.path().to_str().unwrap(), "sync", true, None).is_err(),
            "doublon refuse"
        );
        assert!(
            add(&db, data.path().to_str().unwrap(), "sync", true, None).is_err(),
            "data folder refused"
        );
        assert!(add(&db, "/nulle/part", "sync", true, None).is_err());
        settle();
        let r = scan(&db, &id).unwrap();
        assert_eq!(r["added"], 1, "{r}");
        let c = db.conn().unwrap();
        let (status, watch_id): (String, String) = c
            .query_row("SELECT status,watch_id FROM documents", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(
            (status.as_str(), watch_id.as_str()),
            ("queued", id.as_str())
        );
        assert_eq!(scan(&db, &id).unwrap()["added"], 0);
        c.execute("UPDATE documents SET status='ready'", [])
            .unwrap();
        std::fs::write(folder.path().join("note.md"), "bonjour, monde entier").unwrap();
        settle();
        let r = scan(&db, &id).unwrap();
        assert_eq!(r["updated"], 1, "{r}");
        let (status, stage): (String, String) = c
            .query_row("SELECT status,stage FROM documents", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(status, "queued");
        assert_eq!(stage, "Changed in the watch");
        std::fs::remove_file(folder.path().join("note.md")).unwrap();
        scan(&db, &id).unwrap();
        let tracked: i64 = c
            .query_row("SELECT count(*) FROM watch_files", [], |r| r.get(0))
            .unwrap();
        let docs: i64 = c
            .query_row("SELECT count(*) FROM documents", [], |r| r.get(0))
            .unwrap();
        assert_eq!((tracked, docs), (0, 1));
        update(&db, &id, None, Some("hoover"), None, Some(None)).unwrap();
        std::fs::write(folder.path().join("depot.txt"), "aspire-moi").unwrap();
        settle();
        let r = scan(&db, &id).unwrap();
        assert_eq!(r["moved"], 1, "{r}");
        assert!(
            !folder.path().join("depot.txt").exists(),
            "the watched folder is empty"
        );
        let source: String = c
            .query_row(
                "SELECT source FROM documents WHERE name='depot.txt'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(
            source.starts_with(data.path().join("imports").to_str().unwrap()),
            "{source}"
        );
        assert_eq!(std::fs::read_to_string(&source).unwrap(), "aspire-moi");
        let r = remove(&db, &id, false).unwrap();
        assert_eq!(r["documents"], 2);
        let orphan: i64 = c
            .query_row(
                "SELECT count(*) FROM documents WHERE watch_id IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(orphan, 2);
    }
}
