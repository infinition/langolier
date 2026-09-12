use crate::db::{err, now, Db, Res, Settings};
use rusqlite::params;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const THEMES: [&str; 6] = ["nuit", "clair", "sauge", "sable", "encre", "corail"];
/// Settings fields a profile may override.
pub const OVERRIDABLE: [&str; 13] = [
    "provider",
    "endpoint",
    "api_key",
    "model",
    "temperature",
    "top_k",
    "max_tokens",
    "strict_grounding",
    "abstain_text",
    "rerank_enabled",
    "rerank_threshold",
    "verify_answer",
    "context_size",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Assistant {
    pub id: String,
    pub name: String,
    pub mission: String,
    pub welcome: String,
    /// Data URL image; empty falls back to the name initial.
    pub avatar: String,
    pub theme: String,
    /// {"all":true} or {"watches":[...],"documents":[...]}.
    pub scope: Value,
    /// Subset of Settings; see OVERRIDABLE.
    pub overrides: Value,
    /// Does the exported chatbot list cited passages under the answer?
    pub show_sources: bool,
    /// File names replaced by Source 1, Source 2 everywhere.
    pub hide_source_names: bool,
    /// Token cap per conversation; 0 is unlimited.
    pub max_conversation_tokens: i64,
    /// Token cap per day across conversations; 0 is unlimited.
    pub daily_token_budget: i64,
    /// Telegram bot token; empty disables the bridge.
    pub telegram_token: String,
    /// Telegram whitelist: ids or @handles; empty means everyone.
    pub telegram_allowed: String,
    pub created: i64,
}
impl Default for Assistant {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "Assistant".into(),
            mission: String::new(),
            welcome: String::new(),
            avatar: String::new(),
            theme: "nuit".into(),
            scope: json!({"all": true}),
            overrides: json!({}),
            show_sources: true,
            hide_source_names: false,
            max_conversation_tokens: 0,
            daily_token_budget: 0,
            telegram_token: String::new(),
            telegram_allowed: String::new(),
            created: 0,
        }
    }
}
pub fn schema() -> &'static str {
    "CREATE TABLE IF NOT EXISTS assistants(id TEXT PRIMARY KEY,name TEXT NOT NULL,mission TEXT NOT NULL DEFAULT '',welcome TEXT NOT NULL DEFAULT '',avatar TEXT NOT NULL DEFAULT '',theme TEXT NOT NULL DEFAULT 'nuit',scope TEXT NOT NULL DEFAULT '{\"all\":true}',overrides TEXT NOT NULL DEFAULT '{}',created INTEGER NOT NULL);"
}
pub fn migrate(c: &rusqlite::Connection) -> Res<()> {
    let has: i64 = c
        .query_row(
            "SELECT count(*) FROM pragma_table_info('conversations') WHERE name='assistant_id'",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    if has == 0 {
        c.execute_batch("ALTER TABLE conversations ADD COLUMN assistant_id TEXT;")
            .map_err(err)?;
    }
    let has: i64 = c
        .query_row(
            "SELECT count(*) FROM pragma_table_info('assistants') WHERE name='show_sources'",
            [],
            |r| r.get(0),
        )
        .map_err(err)?;
    if has == 0 {
        c.execute_batch(
            "ALTER TABLE assistants ADD COLUMN show_sources INTEGER NOT NULL DEFAULT 1;",
        )
        .map_err(err)?;
    }
    for (col, ddl) in [
        (
            "hide_source_names",
            "ALTER TABLE assistants ADD COLUMN hide_source_names INTEGER NOT NULL DEFAULT 0;",
        ),
        (
            "max_conversation_tokens",
            "ALTER TABLE assistants ADD COLUMN max_conversation_tokens INTEGER NOT NULL DEFAULT 0;",
        ),
        (
            "daily_token_budget",
            "ALTER TABLE assistants ADD COLUMN daily_token_budget INTEGER NOT NULL DEFAULT 0;",
        ),
        (
            "telegram_token",
            "ALTER TABLE assistants ADD COLUMN telegram_token TEXT NOT NULL DEFAULT '';",
        ),
        (
            "telegram_allowed",
            "ALTER TABLE assistants ADD COLUMN telegram_allowed TEXT NOT NULL DEFAULT '';",
        ),
    ] {
        let has: i64 = c
            .query_row(
                "SELECT count(*) FROM pragma_table_info('assistants') WHERE name=?1",
                [col],
                |r| r.get(0),
            )
            .map_err(err)?;
        if has == 0 {
            c.execute_batch(ddl).map_err(err)?;
        }
    }
    Ok(())
}
fn row(r: &rusqlite::Row) -> rusqlite::Result<Assistant> {
    Ok(Assistant {
        id: r.get(0)?,
        name: r.get(1)?,
        mission: r.get(2)?,
        welcome: r.get(3)?,
        avatar: r.get(4)?,
        theme: r.get(5)?,
        scope: serde_json::from_str(&r.get::<_, String>(6)?).unwrap_or(json!({"all": true})),
        overrides: serde_json::from_str(&r.get::<_, String>(7)?).unwrap_or(json!({})),
        created: r.get(8)?,
        show_sources: r.get::<_, i64>(9)? != 0,
        hide_source_names: r.get::<_, i64>(10)? != 0,
        max_conversation_tokens: r.get(11)?,
        daily_token_budget: r.get(12)?,
        telegram_token: r.get(13)?,
        telegram_allowed: r.get(14)?,
    })
}
const COLS: &str = "id,name,mission,welcome,avatar,theme,scope,overrides,created,show_sources,hide_source_names,max_conversation_tokens,daily_token_budget,telegram_token,telegram_allowed";
pub fn list(db: &Db) -> Res<Vec<Assistant>> {
    let c = db.conn()?;
    let mut st = c
        .prepare(&format!("SELECT {COLS} FROM assistants ORDER BY created"))
        .map_err(err)?;
    let out = st
        .query_map([], row)
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    Ok(out)
}
pub fn get(db: &Db, id: &str) -> Res<Option<Assistant>> {
    if id.is_empty() {
        return Ok(None);
    }
    let c = db.conn()?;
    match c.query_row(
        &format!("SELECT {COLS} FROM assistants WHERE id=?1"),
        [id],
        row,
    ) {
        Ok(a) => Ok(Some(a)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(err(e)),
    }
}
pub fn save(db: &Db, mut a: Assistant) -> Res<Assistant> {
    a.name = a.name.trim().chars().take(60).collect();
    if a.name.is_empty() {
        return Err("Give the assistant a name.".into());
    }
    if a.mission.chars().count() > 6000 || a.welcome.chars().count() > 600 {
        return Err("Mission capped at 6,000 characters, welcome at 600.".into());
    }
    if !THEMES.contains(&a.theme.as_str()) {
        a.theme = "nuit".into();
    }
    if !a.avatar.is_empty() && (!a.avatar.starts_with("data:image/") || a.avatar.len() > 600_000) {
        return Err("The avatar must be an image (PNG or JPEG) under 450 KB.".into());
    }
    if !a.overrides.is_object() {
        a.overrides = json!({});
    }
    let clean: serde_json::Map<String, Value> = a
        .overrides
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, v)| OVERRIDABLE.contains(&k.as_str()) && !v.is_null())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    a.overrides = Value::Object(clean);
    // Overrides must still produce valid settings.
    effective(&db.settings()?, Some(&a))?;
    if a.id.is_empty() {
        a.id = uuid::Uuid::new_v4().to_string();
        a.created = now();
    }
    db.conn()?
        .execute(
            "INSERT INTO assistants(id,name,mission,welcome,avatar,theme,scope,overrides,created,show_sources,hide_source_names,max_conversation_tokens,daily_token_budget,telegram_token,telegram_allowed) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15) ON CONFLICT(id) DO UPDATE SET name=excluded.name,mission=excluded.mission,welcome=excluded.welcome,avatar=excluded.avatar,theme=excluded.theme,scope=excluded.scope,overrides=excluded.overrides,show_sources=excluded.show_sources,hide_source_names=excluded.hide_source_names,max_conversation_tokens=excluded.max_conversation_tokens,daily_token_budget=excluded.daily_token_budget,telegram_token=excluded.telegram_token,telegram_allowed=excluded.telegram_allowed",
            params![a.id, a.name, a.mission, a.welcome, a.avatar, a.theme, a.scope.to_string(), a.overrides.to_string(), if a.created == 0 { now() } else { a.created }, a.show_sources as i64, a.hide_source_names as i64, a.max_conversation_tokens.max(0), a.daily_token_budget.max(0), a.telegram_token.trim(), a.telegram_allowed.trim()],
        )
        .map_err(err)?;
    Ok(a)
}
pub fn remove(db: &Db, id: &str) -> Res<()> {
    let c = db.conn()?;
    c.execute(
        "UPDATE conversations SET assistant_id=NULL WHERE assistant_id=?1",
        [id],
    )
    .map_err(err)?;
    let n = c
        .execute("DELETE FROM assistants WHERE id=?1", [id])
        .map_err(err)?;
    if n == 0 {
        return Err("Assistant introuvable".into());
    }
    Ok(())
}
/// Global settings covered by the profile's overrides.
pub fn effective(base: &Settings, a: Option<&Assistant>) -> Res<Settings> {
    let Some(a) = a else { return Ok(base.clone()) };
    let mut v = serde_json::to_value(base).map_err(err)?;
    if let (Some(obj), Some(over)) = (v.as_object_mut(), a.overrides.as_object()) {
        for (k, val) in over {
            if OVERRIDABLE.contains(&k.as_str()) {
                obj.insert(k.clone(), val.clone());
            }
        }
    }
    let s: Settings =
        serde_json::from_value(v).map_err(|e| format!("Surcharges invalides : {e}"))?;
    if !crate::llm::PROVIDERS.contains(&s.provider.as_str()) || s.model.trim().is_empty() {
        return Err("Invalid provider or model in the profile.".into());
    }
    if !(1..=10).contains(&s.top_k)
        || !(0.0..=2.0).contains(&s.temperature)
        || !(256..=32768).contains(&s.max_tokens)
    {
        return Err("Profile settings out of range.".into());
    }
    Ok(s)
}
/// Document ids the profile may read; None means the whole memory.
pub fn scope_docs(db: &Db, a: Option<&Assistant>) -> Res<Option<Vec<String>>> {
    let Some(a) = a else { return Ok(None) };
    if a.scope["all"] == true {
        return Ok(None);
    }
    let c = db.conn()?;
    let mut ids: Vec<String> = a.scope["documents"]
        .as_array()
        .map(|v| {
            v.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    if let Some(watches) = a.scope["watches"].as_array() {
        let list: Vec<String> = watches
            .iter()
            .filter_map(|x| x.as_str().map(str::to_string))
            .collect();
        if !list.is_empty() {
            let mut st = c
                .prepare(
                    "SELECT id FROM documents WHERE watch_id IN (SELECT value FROM json_each(?1))",
                )
                .map_err(err)?;
            let more = st
                .query_map([serde_json::to_string(&list).map_err(err)?], |r| {
                    r.get::<_, String>(0)
                })
                .map_err(err)?
                .filter_map(Result::ok);
            ids.extend(more);
        }
    }
    ids.sort();
    ids.dedup();
    Ok(Some(ids))
}
pub fn persona(a: Option<&Assistant>) -> String {
    match a {
        Some(a) if !a.mission.trim().is_empty() => format!(
            "You are {}. Your mission, set by your administrator: {}\n\n",
            a.name.trim(),
            a.mission.trim()
        ),
        Some(a) => format!("You are {}.\n\n", a.name.trim()),
        None => "You are Langolier.\n\n".into(),
    }
}
