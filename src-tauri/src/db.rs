use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
pub type Res<T> = Result<T, String>;
pub fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(default)]
pub struct Settings {
    pub provider: String,
    pub endpoint: String,
    pub model: String,
    pub embedding_endpoint: String,
    pub embedding_model: String,
    pub whisper_model: String,
    pub top_k: usize,
    pub temperature: f32,
    pub context_size: usize,
    /// Key for the remote chat provider.
    pub api_key: String,
    /// Maximum answer length, in tokens.
    pub max_tokens: usize,
    /// Minimum cosine for a passage to count in dense search.
    pub min_dense_score: f32,
    /// Minimum BM25 score; 0 disables it.
    pub min_lexical_score: f32,
    /// Second pass by a judge model, with an abstention threshold.
    pub rerank_enabled: bool,
    /// Judge model; empty means the chat model.
    pub rerank_model: String,
    pub rerank_candidates: usize,
    pub rerank_threshold: f32,
    pub strict_grounding: bool,
    pub abstain_text: String,
    /// After-the-fact check that the answer is supported by the passages.
    pub verify_answer: bool,
    /// Global palette shortcut, in Tauri accelerator syntax.
    pub shortcut: String,
    /// Watch scan interval, in seconds.
    pub watch_interval: u64,
    /// Active assistant profile; empty means bare Langolier.
    pub active_assistant: String,
    pub ingestion_paused: bool,
    /// Minutes before the embedded engine unloads idle models; 0 keeps them.
    pub engine_idle_minutes: u64,
    /// Icon in the menu bar or system tray, with Open, Ask and Quit.
    pub tray_icon: bool,
    /// Start with the window hidden; the tray icon and the shortcut remain.
    pub start_hidden: bool,
    pub launch_at_login: bool,
}
use crate::llm::EMBEDDED;
pub const SHORTCUT: &str = "CommandOrControl+Shift+Space";
pub const ABSTAIN: &str = "I do not have that information.";
/// The first French default named the retrieval machinery and the model
/// echoed it; it is replaced on start by its neutral French successor.
pub const ABSTAIN_LEGACY: &str = "Je ne trouve pas cette information dans les sources indexées.";
pub const ABSTAIN_FR: &str = "Je n’ai pas cette information.";
impl Default for Settings {
    fn default() -> Self {
        Self {
            provider: EMBEDDED.into(),
            endpoint: "http://127.0.0.1:11434".into(),
            model: "qwen3:4b-instruct".into(),
            embedding_endpoint: EMBEDDED.into(),
            embedding_model: "embeddinggemma".into(),
            whisper_model: String::new(),
            top_k: 6,
            temperature: 0.2,
            context_size: 8192,
            api_key: String::new(),
            max_tokens: 2048,
            min_dense_score: 0.35,
            min_lexical_score: 0.0,
            rerank_enabled: true,
            rerank_model: String::new(),
            rerank_candidates: 20,
            rerank_threshold: 0.35,
            strict_grounding: true,
            abstain_text: ABSTAIN.into(),
            verify_answer: false,
            shortcut: SHORTCUT.into(),
            watch_interval: 30,
            active_assistant: String::new(),
            ingestion_paused: false,
            engine_idle_minutes: 5,
            tray_icon: false,
            start_hidden: false,
            launch_at_login: false,
        }
    }
}
/// Connections kept warm between calls. Reopening one drops the page cache,
/// and a dense search then rereads every vector from disk.
const POOL_SIZE: usize = 8;
/// A pooled connection, handed back on drop. It behaves like a Connection,
/// and like one it must not be held across an await: it is not Sync.
pub struct Pooled {
    conn: Option<Connection>,
    pool: Arc<Mutex<Vec<Connection>>>,
}
impl Deref for Pooled {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        self.conn.as_ref().expect("connection taken")
    }
}
impl DerefMut for Pooled {
    fn deref_mut(&mut self) -> &mut Connection {
        self.conn.as_mut().expect("connection taken")
    }
}
impl Drop for Pooled {
    fn drop(&mut self) {
        if let Some(c) = self.conn.take() {
            if let Ok(mut pool) = self.pool.lock() {
                if pool.len() < POOL_SIZE {
                    pool.push(c)
                }
            }
        }
    }
}
#[derive(Clone)]
pub struct Db {
    pub root: PathBuf,
    pool: Arc<Mutex<Vec<Connection>>>,
}
/// Vectors written before storage normalization carry their original length,
/// which a plain dot product would read as a score. One pass, once.
fn normalize_vectors(c: &Connection) -> Res<()> {
    let version: i64 = c
        .query_row("PRAGMA user_version", [], |r| r.get(0))
        .map_err(err)?;
    if version >= 1 {
        return Ok(());
    }
    let rows: Vec<(i64, Vec<u8>)> = {
        let mut st = c
            .prepare("SELECT id,embedding FROM chunks WHERE embedding IS NOT NULL")
            .map_err(err)?;
        let found = st
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        found
    };
    c.execute_batch("BEGIN;").map_err(err)?;
    for (id, blob) in rows {
        let v: Vec<f32> = blob
            .as_chunks::<4>()
            .0
            .iter()
            .map(|b| f32::from_le_bytes(*b))
            .collect();
        c.execute(
            "UPDATE chunks SET embedding=?2 WHERE id=?1",
            params![id, crate::rag::encode(&v)],
        )
        .map_err(err)?;
    }
    c.execute_batch("COMMIT; PRAGMA user_version=1;")
        .map_err(err)
}
impl Db {
    pub fn new(root: &Path) -> Res<Self> {
        std::fs::create_dir_all(root.join("media")).map_err(err)?;
        let db = Self {
            root: root.into(),
            pool: Arc::new(Mutex::new(vec![])),
        };
        let c = db.conn()?;
        c.execute_batch("PRAGMA journal_mode=WAL;
 CREATE TABLE IF NOT EXISTS settings(id INTEGER PRIMARY KEY CHECK(id=1),value TEXT NOT NULL);
 CREATE TABLE IF NOT EXISTS documents(id TEXT PRIMARY KEY,name TEXT NOT NULL,source TEXT NOT NULL,kind TEXT NOT NULL,status TEXT NOT NULL DEFAULT 'queued',stage TEXT NOT NULL DEFAULT 'Queued',error TEXT,hash TEXT,language TEXT,bytes INTEGER DEFAULT 0,created INTEGER NOT NULL,updated INTEGER NOT NULL);
 CREATE UNIQUE INDEX IF NOT EXISTS dedup_hash ON documents(hash) WHERE hash IS NOT NULL;
 CREATE TABLE IF NOT EXISTS chunks(id INTEGER PRIMARY KEY,doc_id TEXT NOT NULL REFERENCES documents(id) ON DELETE CASCADE,ordinal INTEGER NOT NULL,text TEXT NOT NULL,locator TEXT NOT NULL,embedding BLOB,embedding_model TEXT,UNIQUE(doc_id,ordinal));
 CREATE VIRTUAL TABLE IF NOT EXISTS chunks_fts USING fts5(text,content='chunks',content_rowid='id',tokenize='unicode61 remove_diacritics 2');
 CREATE TRIGGER IF NOT EXISTS chunks_ai AFTER INSERT ON chunks BEGIN INSERT INTO chunks_fts(rowid,text) VALUES(new.id,new.text); END;
 CREATE TRIGGER IF NOT EXISTS chunks_ad AFTER DELETE ON chunks BEGIN INSERT INTO chunks_fts(chunks_fts,rowid,text) VALUES('delete',old.id,old.text); END;
 CREATE TRIGGER IF NOT EXISTS chunks_au AFTER UPDATE OF text ON chunks BEGIN INSERT INTO chunks_fts(chunks_fts,rowid,text) VALUES('delete',old.id,old.text); INSERT INTO chunks_fts(rowid,text) VALUES(new.id,new.text); END;
 CREATE TABLE IF NOT EXISTS conversations(id TEXT PRIMARY KEY,title TEXT NOT NULL,created INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS messages(id TEXT PRIMARY KEY,conversation_id TEXT REFERENCES conversations(id) ON DELETE CASCADE,role TEXT NOT NULL,content TEXT NOT NULL,sources TEXT NOT NULL DEFAULT '[]',created INTEGER NOT NULL,feedback INTEGER);
 CREATE TABLE IF NOT EXISTS runs(id TEXT PRIMARY KEY,kind TEXT NOT NULL,model TEXT NOT NULL,status TEXT NOT NULL,latency_ms INTEGER NOT NULL,tokens INTEGER DEFAULT 0,tps REAL,details TEXT NOT NULL,created INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS api_keys(id TEXT PRIMARY KEY,label TEXT NOT NULL UNIQUE,provider TEXT NOT NULL DEFAULT '',key TEXT NOT NULL,created INTEGER NOT NULL);
 CREATE TABLE IF NOT EXISTS evaluations(id TEXT PRIMARY KEY,question TEXT NOT NULL,expected_document TEXT NOT NULL);
 CREATE INDEX IF NOT EXISTS chunks_doc ON chunks(doc_id);
 CREATE INDEX IF NOT EXISTS docs_state ON documents(status,created);
 UPDATE documents SET status='queued',stage='Resumed after interruption' WHERE status='processing';
 UPDATE documents SET stage=CASE stage WHEN 'Index hybride' THEN 'Hybrid index' WHEN 'Index lexical' THEN 'Lexical index' WHEN 'À vérifier' THEN 'Needs checking' WHEN 'En attente' THEN 'Queued' WHEN 'Préparation' THEN 'Preparing' ELSE stage END WHERE stage IN ('Index hybride','Index lexical','À vérifier','En attente','Préparation');
 UPDATE documents SET status='duplicate',stage='Duplicate',error='Duplicate of '||substr(error,length('Doublon de contenu : ')+1) WHERE status='error' AND error LIKE 'Doublon de contenu : %';").map_err(err)?;
        normalize_vectors(&c)?;
        c.execute_batch(crate::watch::schema()).map_err(err)?;
        crate::watch::migrate(&c)?;
        c.execute_batch(crate::assistant::schema()).map_err(err)?;
        crate::assistant::migrate(&c)?;
        c.execute_batch(crate::telegram::schema()).map_err(err)?;
        drop(c);
        let mut s = db.settings()?;
        if s.abstain_text.trim() == ABSTAIN_LEGACY {
            s.abstain_text = ABSTAIN_FR.into();
            db.set_settings(&s)?;
        }
        Ok(db)
    }
    pub fn conn(&self) -> Res<Pooled> {
        let reused = self.pool.lock().ok().and_then(|mut p| p.pop());
        let conn = match reused {
            Some(c) => c,
            None => {
                let c = Connection::open(self.root.join("langolier.sqlite3")).map_err(err)?;
                c.busy_timeout(std::time::Duration::from_secs(15))
                    .map_err(err)?;
                // A large page cache and a read-only memory map keep the vectors
                // resident: a dense search walks all of them on every question.
                c.execute_batch(
                    "PRAGMA foreign_keys=ON;
 PRAGMA synchronous=NORMAL;
 PRAGMA temp_store=MEMORY;
 PRAGMA cache_size=-32768;
 PRAGMA mmap_size=268435456;",
                )
                .map_err(err)?;
                c
            }
        };
        Ok(Pooled {
            conn: Some(conn),
            pool: self.pool.clone(),
        })
    }
    pub fn settings(&self) -> Res<Settings> {
        let c = self.conn()?;
        let raw: Result<String, _> =
            c.query_row("SELECT value FROM settings WHERE id=1", [], |r| r.get(0));
        match raw {
            Ok(v) => serde_json::from_str(&v).map_err(err),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(Settings::default()),
            Err(e) => Err(err(e)),
        }
    }
    pub fn set_settings(&self, s: &Settings) -> Res<()> {
        self.conn()?.execute("INSERT INTO settings VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET value=excluded.value",[serde_json::to_string(s).map_err(err)?]).map_err(err)?;
        Ok(())
    }
    pub fn stage(&self, id: &str, stage: &str) -> Res<()> {
        self.conn()?
            .execute(
                "UPDATE documents SET stage=?2,updated=?3 WHERE id=?1",
                params![id, stage, now()],
            )
            .map_err(err)?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn log(
        &self,
        kind: &str,
        model: &str,
        status: &str,
        ms: u64,
        tokens: u64,
        tps: Option<f64>,
        details: &serde_json::Value,
    ) -> Res<()> {
        self.conn()?
            .execute(
                "INSERT INTO runs VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    uuid::Uuid::new_v4().to_string(),
                    kind,
                    model,
                    status,
                    ms,
                    tokens,
                    tps,
                    details.to_string(),
                    now()
                ],
            )
            .map_err(err)?;
        Ok(())
    }
}
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
pub fn json_rows(db: &Db, sql: &str) -> Res<Vec<serde_json::Value>> {
    let c = db.conn()?;
    let mut q = c.prepare(sql).map_err(err)?;
    let names = q
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let r = q
        .query_map([], |r| {
            let mut m = serde_json::Map::new();
            for (i, n) in names.iter().enumerate() {
                let v = match r.get_ref(i)? {
                    rusqlite::types::ValueRef::Null => serde_json::Value::Null,
                    rusqlite::types::ValueRef::Integer(v) => v.into(),
                    rusqlite::types::ValueRef::Real(v) => serde_json::json!(v),
                    rusqlite::types::ValueRef::Text(v) => {
                        String::from_utf8_lossy(v).to_string().into()
                    }
                    _ => serde_json::Value::Null,
                };
                m.insert(n.clone(), v);
            }
            Ok(serde_json::Value::Object(m))
        })
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    Ok(r)
}
