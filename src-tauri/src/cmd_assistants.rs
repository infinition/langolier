//! Tauri commands for assistants, their API keys, and the Telegram bridge.
use crate::db::{err, now, Res};
use crate::desktop::AppState;
use rusqlite::params;
use serde_json::{json, Value};
use tauri::State;

#[tauri::command]
pub async fn telegram_check(token: String) -> Res<Value> {
    crate::telegram::check(&token).await
}
fn key_hint(key: &str) -> String {
    let n = key.chars().count();
    if n <= 6 {
        return "•".repeat(n);
    }
    format!(
        "{}…{}",
        key.chars().take(3).collect::<String>(),
        key.chars().skip(n - 4).collect::<String>()
    )
}
#[tauri::command]
pub fn api_keys(state: State<AppState>) -> Res<Vec<Value>> {
    let c = state.db.conn()?;
    let mut st = c
        .prepare("SELECT id,label,provider,key,created FROM api_keys ORDER BY label")
        .map_err(err)?;
    let rows = st
        .query_map([], |r| {
            let key: String = r.get(3)?;
            Ok(json!({"id": r.get::<_, String>(0)?, "label": r.get::<_, String>(1)?, "provider": r.get::<_, String>(2)?, "hint": key_hint(&key), "created": r.get::<_, i64>(4)?}))
        })
        .map_err(err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(err)?;
    Ok(rows)
}
#[tauri::command]
pub fn save_api_key(
    state: State<AppState>,
    label: String,
    provider: String,
    key: String,
) -> Res<Value> {
    let (label, key) = (label.trim().to_string(), key.trim().to_string());
    if label.is_empty() || label.chars().count() > 60 {
        return Err("Give this key a short name.".into());
    }
    if key.len() < 8 || key.len() > 400 {
        return Err("This key is not a plausible length.".into());
    }
    let id = uuid::Uuid::new_v4().to_string();
    state
        .db
        .conn()?
        .execute(
            "INSERT INTO api_keys(id,label,provider,key,created) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(label) DO UPDATE SET provider=excluded.provider,key=excluded.key",
            params![id, label, provider.trim(), key, now()],
        )
        .map_err(err)?;
    Ok(json!({"label": label, "hint": key_hint(&key)}))
}
#[tauri::command]
pub fn delete_api_key(state: State<AppState>, id: String) -> Res<()> {
    let n = state
        .db
        .conn()?
        .execute("DELETE FROM api_keys WHERE id=?1", [id])
        .map_err(err)?;
    if n == 0 {
        return Err("Key not found".into());
    }
    Ok(())
}
/// Returns the full key to drop into settings or a profile.
#[tauri::command]
pub fn use_api_key(state: State<AppState>, id: String) -> Res<String> {
    state
        .db
        .conn()?
        .query_row("SELECT key FROM api_keys WHERE id=?1", [id], |r| r.get(0))
        .map_err(|_| "Key not found".to_string())
}
#[tauri::command]
pub fn save_assistant(
    state: State<AppState>,
    assistant: crate::assistant::Assistant,
) -> Res<crate::assistant::Assistant> {
    crate::assistant::save(&state.db, assistant)
}
#[tauri::command]
pub fn delete_assistant(state: State<AppState>, id: String) -> Res<()> {
    let mut s = state.db.settings()?;
    if s.active_assistant == id {
        s.active_assistant.clear();
        state.db.set_settings(&s)?;
    }
    crate::assistant::remove(&state.db, &id)
}
#[tauri::command]
pub fn set_active_assistant(state: State<AppState>, id: String) -> Res<()> {
    if !id.is_empty() && crate::assistant::get(&state.db, &id)?.is_none() {
        return Err("Assistant not found".into());
    }
    let mut s = state.db.settings()?;
    s.active_assistant = id;
    state.db.set_settings(&s)
}