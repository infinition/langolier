//! Tauri commands for watched folders.
use crate::db::{err, Res};
use crate::desktop::AppState;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn add_watch(
    state: State<AppState>,
    path: String,
    mode: String,
    recursive: bool,
    interval: Option<i64>,
) -> Res<Value> {
    crate::watch::add(
        &state.db,
        &path,
        &mode,
        recursive,
        interval.filter(|v| *v > 0),
    )
}
#[tauri::command]
pub fn update_watch(
    state: State<AppState>,
    id: String,
    enabled: Option<bool>,
    mode: Option<String>,
    recursive: Option<bool>,
    interval: Option<i64>,
) -> Res<()> {
    crate::watch::update(
        &state.db,
        &id,
        enabled,
        mode.as_deref(),
        recursive,
        interval.map(|v| (v > 0).then_some(v)),
    )
}
#[tauri::command]
pub fn delete_watch(state: State<AppState>, id: String, forget: bool) -> Res<Value> {
    crate::watch::remove(&state.db, &id, forget)
}
#[tauri::command]
pub async fn scan_watch(state: State<'_, AppState>, id: String) -> Res<Value> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || crate::watch::scan(&db, &id))
        .await
        .map_err(err)?
}
