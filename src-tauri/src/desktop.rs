//! Desktop application: Tauri commands and window handling.
use crate::db::{err, now, Db, Res, Settings};
use rusqlite::params;
use serde_json::{json, Value};
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Instant,
};
use tauri::{Emitter, Manager, State};
use tauri_plugin_autostart::ManagerExt as _;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// The tray icon lives here so a settings change can create or drop it.
struct Tray(std::sync::Mutex<Option<tauri::tray::TrayIcon>>);
fn show_main(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
}
fn apply_tray(app: &tauri::AppHandle, enabled: bool) -> Res<()> {
    use tauri::menu::{MenuBuilder, MenuItemBuilder};
    let slot = app.state::<Tray>();
    let mut slot = slot.0.lock().map_err(|_| "tray")?;
    if !enabled {
        *slot = None;
        return Ok(());
    }
    if slot.is_some() {
        return Ok(());
    }
    let open = MenuItemBuilder::with_id("open", "Open Langolier")
        .build(app)
        .map_err(err)?;
    let ask = MenuItemBuilder::with_id("ask", "Ask")
        .build(app)
        .map_err(err)?;
    let quit = MenuItemBuilder::with_id("quit", "Quit")
        .build(app)
        .map_err(err)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open, &ask])
        .separator()
        .item(&quit)
        .build()
        .map_err(err)?;
    let mut builder = tauri::tray::TrayIconBuilder::with_id("langolier")
        .menu(&menu)
        .show_menu_on_left_click(true)
        .tooltip("Langolier")
        .on_menu_event(|app, e| match e.id().as_ref() {
            "open" => show_main(app),
            "ask" => toggle_palette(app),
            "quit" => {
                crate::engine::shutdown();
                app.exit(0)
            }
            _ => {}
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone()).icon_as_template(false);
    }
    *slot = Some(builder.build(app).map_err(err)?);
    Ok(())
}
fn apply_autostart(app: &tauri::AppHandle, enabled: bool) -> Res<()> {
    let al = app.autolaunch();
    let now = al.is_enabled().unwrap_or(false);
    if enabled && !now {
        al.enable().map_err(err)?;
    } else if !enabled && now {
        al.disable().map_err(err)?;
    }
    Ok(())
}
struct AppState {
    db: Db,
    cancel: Arc<AtomicBool>,
    generation: tokio::sync::Mutex<()>,
}
#[tauri::command]
fn snapshot(state: State<AppState>) -> Res<Value> {
    let db = &state.db;
    Ok(
        json!({"settings":db.settings()?,"documents":crate::db::json_rows(db,"SELECT d.*,count(ch.id) AS chunks,count(ch.embedding) AS embedded FROM documents d LEFT JOIN chunks ch ON ch.doc_id=d.id GROUP BY d.id ORDER BY d.created DESC")?,"conversations":crate::db::json_rows(db,"SELECT id,title,created,coalesce(assistant_id,'') AS assistant_id FROM conversations ORDER BY created DESC")?,"assistants":serde_json::to_value(crate::assistant::list(db)?).map_err(err)?,"watches":crate::db::json_rows(db,"SELECT w.*,count(d.id) AS total,coalesce(sum(d.status='ready'),0) AS ready,coalesce(sum(d.status='error'),0) AS errors,coalesce(sum(d.status IN ('queued','processing')),0) AS pending FROM watches w LEFT JOIN documents d ON d.watch_id=w.id GROUP BY w.id ORDER BY w.created")?,"runs":crate::db::json_rows(db,"SELECT * FROM runs ORDER BY created DESC,rowid DESC LIMIT 100")?,"evaluations":crate::db::json_rows(db,"SELECT * FROM evaluations ORDER BY rowid")?,"totals":crate::db::json_rows(db,"SELECT count(*) AS requests,sum(status='ok') AS successful,avg(latency_ms) AS latency_ms,avg(tps) AS tps,sum(tokens) AS tokens FROM runs WHERE kind='chat'")?,"feedback":crate::db::json_rows(db,"SELECT count(*) AS rated,sum(feedback=1) AS positive FROM messages WHERE feedback IS NOT NULL")?,"data_dir":db.root.display().to_string()}),
    )
}
#[tauri::command]
fn save_settings(app: tauri::AppHandle, state: State<AppState>, settings: Settings) -> Res<()> {
    if !crate::llm::PROVIDERS.contains(&settings.provider.as_str()) {
        return Err("Unsupported provider".into());
    }
    crate::llm::chat_endpoint(&settings)?;
    crate::engine::set_idle_secs(settings.engine_idle_minutes * 60);
    apply_tray(&app, settings.tray_icon || settings.start_hidden)?;
    if let Err(e) = apply_autostart(&app, settings.launch_at_login) {
        eprintln!("Launch at login: {e}");
    }
    if settings.embedding_endpoint.trim() == crate::llm::EMBEDDED {
        if !crate::engine::available(&settings.embedding_model) {
            return Err(format!(
                "GGUF embedding model not found: {}",
                settings.embedding_model
            ));
        }
    } else {
        crate::llm::local_endpoint(&settings.embedding_endpoint)?;
    }
    if !(1..=10).contains(&settings.top_k)
        || !(4096..=32768).contains(&settings.context_size)
        || !settings.temperature.is_finite()
        || !(0.0..=2.0).contains(&settings.temperature)
        || !(256..=32768).contains(&settings.max_tokens)
        || !(5..=3600).contains(&settings.watch_interval)
        || settings.model.trim().is_empty()
        || settings.embedding_model.trim().is_empty()
    {
        return Err("Settings out of range".into());
    }
    if !settings.min_dense_score.is_finite()
        || !(0.0..=1.0).contains(&settings.min_dense_score)
        || !settings.min_lexical_score.is_finite()
        || !(0.0..=100.0).contains(&settings.min_lexical_score)
        || !(1..=50).contains(&settings.rerank_candidates)
        || !settings.rerank_threshold.is_finite()
        || !(0.0..=1.0).contains(&settings.rerank_threshold)
        || settings.abstain_text.trim().is_empty()
        || settings.abstain_text.chars().count() > 400
    {
        return Err("Grounding settings out of range".into());
    }
    settings
        .shortcut
        .parse::<Shortcut>()
        .map_err(|e| format!("Raccourci invalide : {e}"))?;
    state.db.set_settings(&settings)?;
    register_shortcut(&app, &settings.shortcut)
}
/// Rebinds the global shortcut from the settings.
fn register_shortcut(app: &tauri::AppHandle, accel: &str) -> Res<()> {
    let gs = app.global_shortcut();
    gs.unregister_all().map_err(err)?;
    let shortcut: Shortcut = accel
        .parse()
        .map_err(|e| format!("Invalid shortcut: {e}"))?;
    gs.register(shortcut)
        .map_err(|e| format!("Shortcut unavailable ({accel}): {e}"))
}
/// While a recorder captures keys, the global shortcut must not fire.
#[tauri::command]
fn pause_shortcut(app: tauri::AppHandle, state: State<AppState>, paused: bool) -> Res<()> {
    if paused {
        app.global_shortcut().unregister_all().map_err(err)
    } else {
        register_shortcut(&app, &state.db.settings()?.shortcut)
    }
}
const PALETTE: &str = "palette";
/// Borderless floating window, created once then shown or hidden.
fn palette_window(app: &tauri::AppHandle) -> Res<tauri::WebviewWindow> {
    if let Some(w) = app.get_webview_window(PALETTE) {
        return Ok(w);
    }
    let (width, height) = (760.0, 600.0);
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        PALETTE,
        tauri::WebviewUrl::App("index.html#palette".into()),
    )
    .title("Langolier")
    .inner_size(width, height)
    .resizable(false)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .shadow(false)
    .visible(false);
    if let Some(monitor) = app.primary_monitor().ok().flatten() {
        let size = monitor.size();
        let scale = monitor.scale_factor();
        let (mw, mh) = (size.width as f64 / scale, size.height as f64 / scale);
        builder = builder.position((mw - width) / 2.0, (mh * 0.18).max(40.0));
    } else {
        builder = builder.center();
    }
    builder.build().map_err(err)
}
fn toggle_palette(app: &tauri::AppHandle) {
    let Ok(w) = palette_window(app) else { return };
    if w.is_visible().unwrap_or(false) && w.is_focused().unwrap_or(false) {
        let _ = w.hide();
        return;
    }
    let _ = w.show();
    let _ = w.set_focus();
    let _ = w.emit("palette-open", ());
}
#[tauri::command]
fn open_palette(app: tauri::AppHandle) {
    toggle_palette(&app)
}
#[tauri::command]
fn hide_palette(app: tauri::AppHandle) {
    if let Some(w) = app.get_webview_window(PALETTE) {
        let _ = w.hide();
    }
}
#[tauri::command]
async fn health(state: State<'_, AppState>) -> Res<Value> {
    let s = state.db.settings()?;
    let result = crate::llm::models(&s).await;
    let mut deps = serde_json::Map::new();
    for (name, arg) in [
        ("ffmpeg", "-version"),
        ("yt-dlp", "--version"),
        ("whisper-cli", "--help"),
        ("pdftoppm", "-v"),
        ("tesseract", "--version"),
    ] {
        deps.insert(
            name.into(),
            json!(crate::ingest::command(name, &[arg.into()], 15)
                .await
                .is_ok()),
        );
    }
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    Ok(
        json!({"engine_ok":result.is_ok(),"models":result.as_ref().ok(),"error":result.err(),"dependencies":deps,"whisper_model_exists":std::path::Path::new(&s.whisper_model).is_file(),"memory_total":sys.total_memory(),"memory_used":sys.used_memory(),"platform":std::env::consts::OS,"arch":std::env::consts::ARCH}),
    )
}
#[tauri::command]
async fn import_files(state: State<'_, AppState>, paths: Vec<String>) -> Res<Value> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || crate::ingest::queue(&db, paths))
        .await
        .map_err(err)?
}
#[tauri::command]
fn import_text(state: State<AppState>, title: String, text: String) -> Res<Value> {
    crate::ingest::paste(&state.db, &title, &text)
}
#[tauri::command]
fn import_url(state: State<AppState>, url: String) -> Res<Value> {
    crate::ingest::queue_url(&state.db, &url)
}
#[tauri::command]
fn retry_document(state: State<AppState>, id: String) -> Res<()> {
    let c = state.db.conn()?;
    let changed=c.execute("UPDATE documents SET status='queued',stage='Queued',error=NULL,updated=?2 WHERE id=?1 AND status!='processing'",params![id,now()]).map_err(err)?;
    if changed == 0 {
        return Err("Source missing or already processing".into());
    }
    Ok(())
}
#[tauri::command]
fn delete_document(state: State<AppState>, id: String) -> Res<()> {
    let n = state
        .db
        .conn()?
        .execute(
            "DELETE FROM documents WHERE id=?1 AND status!='processing'",
            [id],
        )
        .map_err(err)?;
    if n == 0 {
        return Err("Attendez la fin du traitement avant de retirer cette source.".into());
    }
    Ok(())
}
/// Removes several sources from the index at once.
#[tauri::command]
fn delete_documents(state: State<AppState>, ids: Vec<String>) -> Res<Value> {
    if ids.is_empty() || ids.len() > 20000 {
        return Err("Selection empty or too large.".into());
    }
    let mut c = state.db.conn()?;
    let tx = c.transaction().map_err(err)?;
    let mut removed = 0usize;
    for id in &ids {
        removed += tx
            .execute(
                "DELETE FROM documents WHERE id=?1 AND status!='processing'",
                [id],
            )
            .map_err(err)?;
    }
    tx.commit().map_err(err)?;
    Ok(json!({"removed": removed, "busy": ids.len() - removed}))
}
#[tauri::command]
fn document_chunks(state: State<AppState>, id: String) -> Res<Vec<Value>> {
    let c = state.db.conn()?;
    let mut st = c
        .prepare("SELECT id,text,locator FROM chunks WHERE doc_id=?1 ORDER BY ordinal LIMIT 500")
        .map_err(err)?;
    let rows=st.query_map([id],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"text":r.get::<_,String>(1)?,"locator":r.get::<_,String>(2)?}))).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
    Ok(rows)
}
#[tauri::command]
fn delete_conversation(state: State<AppState>, id: String) -> Res<()> {
    let _guard = state
        .generation
        .try_lock()
        .map_err(|_| "Wait for the current generation to finish.")?;
    let n = state
        .db
        .conn()?
        .execute("DELETE FROM conversations WHERE id=?1", [id])
        .map_err(err)?;
    if n == 0 {
        return Err("Conversation not found".into());
    }
    Ok(())
}
/// Recomputes vectors for edited passages with the same model.
async fn reembed(db: &Db, ids: &[i64]) -> Res<Option<String>> {
    let s = db.settings()?;
    let mut rows = vec![];
    {
        let c = db.conn()?;
        for id in ids {
            let row = c.query_row("SELECT ch.id,d.name,ch.locator,ch.text FROM chunks ch JOIN documents d ON d.id=ch.doc_id WHERE ch.id=?1",[id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?))).map_err(err)?;
            rows.push(row);
        }
        c.execute(
            &format!(
                "UPDATE chunks SET embedding=NULL,embedding_model=NULL WHERE id IN ({})",
                ids.iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            [],
        )
        .map_err(err)?;
    }
    for batch in rows.chunks(16) {
        let texts = batch
            .iter()
            .map(|(_, name, l, t)| format!("Document: {name}\nSection: {l}\n{t}"))
            .collect::<Vec<_>>();
        match crate::llm::embeddings(&s, &texts).await {
            Ok(vectors) => {
                let mut c = db.conn()?;
                let tx = c.transaction().map_err(err)?;
                for ((id, ..), v) in batch.iter().zip(vectors) {
                    tx.execute(
                        "UPDATE chunks SET embedding=?2,embedding_model=?3 WHERE id=?1",
                        params![id, crate::rag::encode(&v), crate::rag::embedding_key(&s)],
                    )
                    .map_err(err)?;
                }
                tx.commit().map_err(err)?;
            }
            Err(e) => {
                return Ok(Some(format!(
                    "Text saved, lexical index up to date. Vectors not recomputed: {e}"
                )))
            }
        }
    }
    Ok(None)
}
#[tauri::command]
async fn update_chunk(state: State<'_, AppState>, id: i64, text: String) -> Res<Value> {
    let text = text.trim().to_string();
    if text.is_empty() || text.chars().count() > 20000 {
        return Err("The passage must hold between 1 and 20,000 characters.".into());
    }
    let n = state
        .db
        .conn()?
        .execute("UPDATE chunks SET text=?2 WHERE id=?1", params![id, text])
        .map_err(err)?;
    if n == 0 {
        return Err("Passage not found".into());
    }
    let warning = reembed(&state.db, &[id]).await?;
    Ok(json!({"updated": 1, "warning": warning}))
}
/// Proposes a fix for one passage without saving anything.
#[tauri::command]
async fn polish_chunk(state: State<'_, AppState>, id: i64) -> Res<Value> {
    let s = state.db.settings()?;
    let (name, text): (String, String) = state
        .db
        .conn()?
        .query_row("SELECT d.name,ch.text FROM chunks ch JOIN documents d ON d.id=ch.doc_id WHERE ch.id=?1", [id], |r| Ok((r.get(0)?, r.get(1)?)))
        .map_err(err)?;
    let proposal = crate::llm::polish(&s, &name, &text, Arc::new(AtomicBool::new(false))).await?;
    Ok(json!({"proposal": proposal, "changed": proposal != text}))
}
/// Fixes a whole source, passage by passage, with progress.
#[tauri::command]
async fn polish_document(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    doc_id: String,
) -> Res<Value> {
    let _guard = state
        .generation
        .try_lock()
        .map_err(|_| "A generation is already running")?;
    state.cancel.store(false, Ordering::Relaxed);
    let s = state.db.settings()?;
    let name: String = state
        .db
        .conn()?
        .query_row("SELECT name FROM documents WHERE id=?1", [&doc_id], |r| {
            r.get(0)
        })
        .map_err(err)?;
    let rows = {
        let c = state.db.conn()?;
        let mut st = c
            .prepare("SELECT id,text FROM chunks WHERE doc_id=?1 ORDER BY ordinal")
            .map_err(err)?;
        let rows = st
            .query_map([&doc_id], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        rows
    };
    let total = rows.len();
    let mut changed = vec![];
    let mut rejected = 0usize;
    for (done, (id, text)) in rows.into_iter().enumerate() {
        if state.cancel.load(Ordering::Relaxed) {
            break;
        }
        let _ = app.emit(
            "polish-progress",
            json!({"doc_id": doc_id, "done": done, "total": total, "changed": changed.len()}),
        );
        match crate::llm::polish(&s, &name, &text, state.cancel.clone()).await {
            Ok(p) if p != text => {
                state
                    .db
                    .conn()?
                    .execute("UPDATE chunks SET text=?2 WHERE id=?1", params![id, p])
                    .map_err(err)?;
                changed.push(id);
            }
            Ok(_) => {}
            Err(e) if e.contains("cancelled") => break,
            Err(_) => rejected += 1,
        }
    }
    let _ = app.emit(
        "polish-progress",
        json!({"doc_id": doc_id, "done": total, "total": total, "changed": changed.len()}),
    );
    let warning = if changed.is_empty() {
        None
    } else {
        reembed(&state.db, &changed).await?
    };
    Ok(
        json!({"chunks": total, "changed": changed.len(), "rejected": rejected, "cancelled": state.cancel.load(Ordering::Relaxed), "warning": warning}),
    )
}
/// Replacement limited to a list of passages.
#[tauri::command]
async fn replace_in_chunks(
    state: State<'_, AppState>,
    ids: Vec<i64>,
    from: String,
    to: String,
) -> Res<Value> {
    if from.is_empty()
        || from.chars().count() > 500
        || to.chars().count() > 500
        || ids.is_empty()
        || ids.len() > 2000
    {
        return Err("Selection empty or text out of range.".into());
    }
    let mut changed = vec![];
    let mut occurrences = 0usize;
    {
        let mut c = state.db.conn()?;
        let tx = c.transaction().map_err(err)?;
        for id in &ids {
            let text: String = tx
                .query_row("SELECT text FROM chunks WHERE id=?1", [id], |r| r.get(0))
                .map_err(err)?;
            let count = text.matches(&from).count();
            if count == 0 {
                continue;
            }
            let replaced = text.replace(&from, &to);
            if replaced.trim().is_empty() {
                return Err("The replacement would empty a passage; delete it instead.".into());
            }
            tx.execute(
                "UPDATE chunks SET text=?2 WHERE id=?1",
                params![id, replaced],
            )
            .map_err(err)?;
            occurrences += count;
            changed.push(*id);
        }
        tx.commit().map_err(err)?;
    }
    let warning = if changed.is_empty() {
        None
    } else {
        reembed(&state.db, &changed).await?
    };
    Ok(json!({"chunks": changed.len(), "occurrences": occurrences, "warning": warning}))
}
#[tauri::command]
fn delete_chunk(state: State<AppState>, id: i64) -> Res<()> {
    let n = state
        .db
        .conn()?
        .execute("DELETE FROM chunks WHERE id=?1", [id])
        .map_err(err)?;
    if n == 0 {
        return Err("Passage not found".into());
    }
    Ok(())
}
/// Replaces text across a source's passages, or across everything.
#[tauri::command]
async fn replace_in_document(
    state: State<'_, AppState>,
    doc_id: String,
    from: String,
    to: String,
    apply: bool,
) -> Res<Value> {
    if from.is_empty() || from.chars().count() > 500 || to.chars().count() > 500 {
        return Err("The text to replace must hold between 1 and 500 characters.".into());
    }
    let mut changed = vec![];
    let mut occurrences = 0usize;
    {
        let c = state.db.conn()?;
        let mut st = c
            .prepare("SELECT id,text FROM chunks WHERE (?1='' OR doc_id=?1) AND instr(text,?2)>0")
            .map_err(err)?;
        let rows = st
            .query_map(params![doc_id, from], |r| {
                Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        for (id, text) in rows {
            let count = text.matches(&from).count();
            if count == 0 {
                continue;
            }
            occurrences += count;
            changed.push((id, text.replace(&from, &to)));
        }
        if apply {
            let mut c = state.db.conn()?;
            let tx = c.transaction().map_err(err)?;
            for (id, text) in &changed {
                if text.trim().is_empty() {
                    return Err("The replacement would empty a passage; delete it instead.".into());
                }
                tx.execute("UPDATE chunks SET text=?2 WHERE id=?1", params![id, text])
                    .map_err(err)?;
            }
            tx.commit().map_err(err)?;
        }
    }
    let ids = changed.iter().map(|(id, _)| *id).collect::<Vec<_>>();
    let warning = if apply && !ids.is_empty() {
        reembed(&state.db, &ids).await?
    } else {
        None
    };
    Ok(
        json!({"chunks": ids.len(), "occurrences": occurrences, "applied": apply, "warning": warning}),
    )
}
#[tauri::command]
fn add_watch(
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
fn update_watch(
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
fn delete_watch(state: State<AppState>, id: String, forget: bool) -> Res<Value> {
    crate::watch::remove(&state.db, &id, forget)
}
#[tauri::command]
async fn scan_watch(state: State<'_, AppState>, id: String) -> Res<Value> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || crate::watch::scan(&db, &id))
        .await
        .map_err(err)?
}
#[tauri::command]
fn new_conversation(state: State<AppState>, assistant_id: Option<String>) -> Res<String> {
    let id = assistant_id.or_else(|| state.db.settings().ok().map(|s| s.active_assistant));
    crate::pipeline::new_conversation(&state.db, id.as_deref())
}
#[tauri::command]
async fn export_assistant(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    id: String,
    dest: String,
    kind: String,
    engine: Option<String>,
) -> Res<Value> {
    let db = state.db.clone();
    let a = crate::assistant::get(&db, &id)?.ok_or("Assistant not found")?;
    let slug = crate::bundle::slug(&a.name);
    let dest = std::path::PathBuf::from(dest.trim());
    if dest.as_os_str().is_empty() {
        return Err("Pick a destination folder.".into());
    }
    let progress = move |stage: &str, done: u64, total: u64| {
        let _ = app.emit(
            "export-progress",
            json!({"stage": stage, "done": done, "total": total}),
        );
    };
    match kind.as_str() {
        "langolier" => {
            std::fs::create_dir_all(&dest).map_err(err)?;
            let file = dest.join(format!("{slug}.langolier"));
            let bytes = crate::bundle::write_langolier(&db, &a, &file)?;
            Ok(json!({"path": file, "bytes": bytes}))
        }
        "kit" => {
            crate::bundle::write_kit(
                &db,
                &a,
                &dest,
                engine.as_deref().unwrap_or("profile"),
                &progress,
            )
            .await
        }
        _ => Err("Type d'export inconnu".into()),
    }
}
#[tauri::command]
fn read_image_data_url(path: String) -> Res<String> {
    let p = std::path::Path::new(&path);
    let ext = p
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mime = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        _ => return Err("Image PNG, JPEG ou WebP attendue.".into()),
    };
    let bytes = std::fs::read(p).map_err(err)?;
    if bytes.len() > 15_000_000 {
        return Err("Image capped at 15 MB.".into());
    }
    Ok(format!("data:{mime};base64,{}", base64_encode(&bytes)))
}
fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len() * 4 / 3 + 4);
    for chunk in bytes.chunks(3) {
        let n = chunk.len();
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let v = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(T[(v >> 18) as usize & 63] as char);
        out.push(T[(v >> 12) as usize & 63] as char);
        out.push(if n > 1 {
            T[(v >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if n > 2 {
            T[v as usize & 63] as char
        } else {
            '='
        });
    }
    out
}
/// Supported embedding models, for the settings panel.
#[tauri::command]
fn embed_models() -> Vec<Value> {
    crate::llm::EMBED_MODELS
        .iter()
        .map(|m| json!({"tag": m.tag, "label": m.label, "dims": m.dims, "gguf_file": m.gguf_file}))
        .collect()
}
#[tauri::command]
async fn telegram_check(token: String) -> Res<Value> {
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
fn api_keys(state: State<AppState>) -> Res<Vec<Value>> {
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
fn save_api_key(
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
fn delete_api_key(state: State<AppState>, id: String) -> Res<()> {
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
fn use_api_key(state: State<AppState>, id: String) -> Res<String> {
    state
        .db
        .conn()?
        .query_row("SELECT key FROM api_keys WHERE id=?1", [id], |r| r.get(0))
        .map_err(|_| "Key not found".to_string())
}
#[tauri::command]
fn save_assistant(
    state: State<AppState>,
    assistant: crate::assistant::Assistant,
) -> Res<crate::assistant::Assistant> {
    crate::assistant::save(&state.db, assistant)
}
#[tauri::command]
fn delete_assistant(state: State<AppState>, id: String) -> Res<()> {
    let mut s = state.db.settings()?;
    if s.active_assistant == id {
        s.active_assistant.clear();
        state.db.set_settings(&s)?;
    }
    crate::assistant::remove(&state.db, &id)
}
#[tauri::command]
fn set_active_assistant(state: State<AppState>, id: String) -> Res<()> {
    if !id.is_empty() && crate::assistant::get(&state.db, &id)?.is_none() {
        return Err("Assistant not found".into());
    }
    let mut s = state.db.settings()?;
    s.active_assistant = id;
    state.db.set_settings(&s)
}
#[tauri::command]
fn messages(state: State<AppState>, id: String) -> Res<Vec<Value>> {
    let c = state.db.conn()?;
    let mut st=c.prepare("SELECT id,role,content,sources,feedback FROM messages WHERE conversation_id=?1 ORDER BY rowid").map_err(err)?;
    let rows=st.query_map([id],|r|{let sources:String=r.get(3)?;Ok(json!({"id":r.get::<_,String>(0)?,"role":r.get::<_,String>(1)?,"content":r.get::<_,String>(2)?,"sources":serde_json::from_str::<Value>(&sources).unwrap_or(json!([])),"feedback":r.get::<_,Option<i64>>(4)?}))}).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
    Ok(rows)
}
#[tauri::command]
fn feedback(state: State<AppState>, id: String, value: i64) -> Res<()> {
    if value != 1 && value != -1 {
        return Err("Invalid rating".into());
    }
    state
        .db
        .conn()?
        .execute(
            "UPDATE messages SET feedback=?2 WHERE id=?1 AND role='assistant'",
            params![id, value],
        )
        .map_err(err)?;
    Ok(())
}
#[tauri::command]
fn cancel_chat(state: State<AppState>) {
    state.cancel.store(true, Ordering::Relaxed);
}
#[tauri::command]
async fn chat(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    conversation_id: String,
    question: String,
    mode: String,
    assisted: bool,
    assistant_id: Option<String>,
) -> Res<Value> {
    let _guard = state
        .generation
        .try_lock()
        .map_err(|_| "A generation is already running")?;
    state.cancel.store(false, Ordering::Relaxed);
    let db = &state.db;
    let base = db.settings()?;
    // The requested profile, else the conversation's, else the active one.
    let wanted = match assistant_id {
        Some(a) => a,
        None => db
            .conn()?
            .query_row(
                "SELECT coalesce(assistant_id,'') FROM conversations WHERE id=?1",
                [&conversation_id],
                |r| r.get::<_, String>(0),
            )
            .unwrap_or_default(),
    };
    let wanted = if wanted.is_empty() {
        base.active_assistant.clone()
    } else {
        wanted
    };
    let profile = crate::assistant::get(db, &wanted)?;
    let s = crate::assistant::effective(&base, profile.as_ref())?;
    let emit = move |kind: &str, payload: Value| {
        let _ = app.emit(&format!("chat-{kind}"), payload);
    };
    crate::pipeline::answer(
        db,
        &s,
        profile.as_ref(),
        &conversation_id,
        &question,
        &mode,
        assisted,
        state.cancel.clone(),
        &emit,
    )
    .await
}
#[tauri::command]
async fn search_sources(state: State<'_, AppState>, query: String, mode: String) -> Res<Value> {
    let s = state.db.settings()?;
    let start = Instant::now();
    let (sources, warning) = crate::rag::retrieve(&state.db, &query, &s, &mode, None).await?;
    Ok(json!({"sources":sources,"warning":warning,"latency_ms":start.elapsed().as_millis()}))
}
#[tauri::command]
fn add_evaluation(state: State<AppState>, question: String, expected_document: String) -> Res<()> {
    if question.trim().is_empty() {
        return Err("Empty question".into());
    }
    let c = state.db.conn()?;
    let n: i64 = c
        .query_row(
            "SELECT count(*) FROM documents WHERE id=?1",
            [&expected_document],
            |r| r.get(0),
        )
        .map_err(err)?;
    if n == 0 {
        return Err("Select an existing source".into());
    }
    c.execute(
        "INSERT INTO evaluations VALUES(?1,?2,?3)",
        params![
            uuid::Uuid::new_v4().to_string(),
            question,
            expected_document
        ],
    )
    .map_err(err)?;
    Ok(())
}
#[tauri::command]
fn delete_evaluation(state: State<AppState>, id: String) -> Res<()> {
    state
        .db
        .conn()?
        .execute("DELETE FROM evaluations WHERE id=?1", [id])
        .map_err(err)?;
    Ok(())
}
#[tauri::command]
async fn run_evaluations(state: State<'_, AppState>) -> Res<Value> {
    let s = state.db.settings()?;
    let cases = crate::db::json_rows(&state.db, "SELECT * FROM evaluations")?;
    if cases.is_empty() {
        return Err("Add at least one reference question.".into());
    }
    let mut reports = vec![];
    for mode in ["lexical", "semantic", "hybrid"] {
        let start = Instant::now();
        let (mut hits, mut rr, mut errors) = (0, 0.0, 0);
        let mut rows = vec![];
        for case in &cases {
            let q = case["question"].as_str().unwrap_or("");
            match crate::rag::retrieve(&state.db, q, &s, mode, None).await {
                Ok((sources, warning)) => {
                    if warning.is_some() {
                        errors += 1;
                    }
                    let rank = sources
                        .iter()
                        .position(|v| v.doc_id == case["expected_document"].as_str().unwrap_or(""));
                    if let Some(i) = rank {
                        hits += 1;
                        rr += 1.0 / (i + 1) as f64
                    }
                    rows.push(json!({"question":q,"rank":rank.map(|r|r+1),"warning":warning}));
                }
                Err(e) => {
                    errors += 1;
                    rows.push(json!({"question":q,"error":e}));
                }
            }
        }
        let report = json!({"mode":mode,"k":s.top_k,"n":cases.len(),"hit_at_k":hits as f64/cases.len()as f64,"mrr":rr/cases.len()as f64,"errors":errors,"latency_ms":start.elapsed().as_millis(),"cases":rows,"embedding_model":s.embedding_model});
        state.db.log(
            "evaluation",
            &s.embedding_model,
            if errors == 0 { "ok" } else { "degraded" },
            start.elapsed().as_millis() as u64,
            0,
            None,
            &report,
        )?;
        reports.push(report);
    }
    Ok(json!(reports))
}
#[tauri::command]
fn export_data(state: State<AppState>, path: String, kind: String) -> Res<Value> {
    let db = &state.db;
    let content = if kind == "training" {
        let c = db.conn()?;
        let mut st=c.prepare("SELECT m.conversation_id,m.rowid,m.content,m.sources FROM messages m WHERE m.role='assistant' AND m.feedback=1 ORDER BY m.rowid").map_err(err)?;
        let pairs = st
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })
            .map_err(err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(err)?;
        let mut lines = vec![];
        for (conv, rowid, answer, raw_sources) in pairs {
            let question: String = c.query_row("SELECT content FROM messages WHERE conversation_id=?1 AND role='user' AND rowid<?2 ORDER BY rowid DESC LIMIT 1", params![conv,rowid], |r|r.get(0)).map_err(err)?;
            let sources: Vec<crate::rag::Source> =
                serde_json::from_str(&raw_sources).map_err(err)?;
            let check = crate::rag::citation_check(&answer, sources.len());
            if check["missing"] == true
                || check["invalid"].as_array().is_some_and(|v| !v.is_empty())
            {
                continue;
            }
            let mut messages = vec![];
            if !sources.is_empty() {
                messages.push(json!({"role":"system","content":format!("Answer using the supplied evidence and cite the source numbers.\n{}",crate::rag::context(&sources))}));
            }
            messages.push(json!({"role":"user","content":question}));
            messages.push(json!({"role":"assistant","content":answer}));
            lines.push(json!({"conversation_id":conv,"messages":messages}).to_string());
        }
        if lines.is_empty() {
            return Err(
                "No approved answer. Rate some answers positively before exporting.".into(),
            );
        }
        lines.join("\n") + "\n"
    } else if kind == "metrics" {
        serde_json::to_string_pretty(&crate::db::json_rows(
            db,
            "SELECT * FROM runs ORDER BY created",
        )?)
        .map_err(err)?
    } else {
        return Err("Type d'export inconnu".into());
    };
    std::fs::write(&path, &content).map_err(err)?;
    Ok(json!({"path":path,"bytes":content.len()}))
}
#[tauri::command]
async fn pull_model(app: tauri::AppHandle, state: State<'_, AppState>, model: String) -> Res<()> {
    if model.is_empty()
        || !model
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_:./".contains(c))
        || model.starts_with('-')
    {
        return Err("Invalid model name".into());
    }
    let s = state.db.settings()?;
    // Curated tags for an embedded role go to the GGUF cache, no Ollama needed.
    let wants_chat_gguf =
        s.provider == crate::llm::EMBEDDED && crate::llm::chat_model(&model).is_some();
    let wants_embed_gguf = s.embedding_endpoint.trim() == crate::llm::EMBEDDED
        && crate::llm::EMBED_MODELS
            .iter()
            .any(|m| m.tag == model.trim());
    if wants_chat_gguf || wants_embed_gguf {
        let progress = |label: &str, done: u64, total: u64| {
            let _ = app.emit(
                "model-progress",
                json!({"status": label, "completed": done, "total": total}),
            );
        };
        if wants_chat_gguf {
            crate::bundle::cached_chat_gguf(&state.db, &model, &progress).await?;
        } else {
            crate::bundle::cached_embed_gguf(&state.db, &model, &progress).await?;
        }
        return Ok(());
    }
    let base = crate::llm::local_endpoint(&s.embedding_endpoint)?;
    let resp = crate::llm::client()?
        .post(format!("{base}/api/pull"))
        .timeout(std::time::Duration::from_secs(7200))
        .json(&json!({"model":model,"stream":true}))
        .send()
        .await
        .map_err(err)?
        .error_for_status()
        .map_err(err)?;
    use futures_util::StreamExt;
    let mut stream = resp.bytes_stream();
    let mut buffer = Vec::new();
    let mut done = false;
    while let Some(part) = stream.next().await {
        buffer.extend_from_slice(&part.map_err(err)?);
        while let Some(p) = buffer.iter().position(|b| *b == b'\n') {
            let raw = buffer.drain(..=p).collect::<Vec<_>>();
            let v: Value = serde_json::from_slice(&raw).map_err(err)?;
            if !v["error"].is_null() {
                return Err(v["error"].to_string());
            }
            if v["status"] == "success" {
                done = true;
            }
            let _ = app.emit("model-progress", v);
        }
    }
    if !done {
        return Err("Download interrupted".into());
    }
    Ok(())
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// generate_context! may appear only once per crate.
pub fn context() -> tauri::Context<tauri::Wry> {
    tauri::generate_context!()
}
pub fn run() {
    // --hidden comes from the login item: start in the tray, no window.
    let hidden_arg = std::env::args().any(|a| a == "--hidden");
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--hidden"]),
        ))
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_palette(app);
                    }
                })
                .build(),
        )
        // macOS behaviour, and any platform with a tray icon: closing hides.
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                let tray = window
                    .app_handle()
                    .state::<Tray>()
                    .0
                    .lock()
                    .map(|t| t.is_some())
                    .unwrap_or(false);
                if cfg!(target_os = "macos") || tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
            tauri::WindowEvent::Focused(false) if window.label() == PALETTE => {
                let _ = window.hide();
            }
            _ => {}
        })
        .setup(move |app| {
            let db = Db::new(&app.path().app_data_dir()?).map_err(std::io::Error::other)?;
            let mut s = db.settings().map_err(std::io::Error::other)?;
            let hidden = hidden_arg || s.start_hidden;
            // The main window is built here, not in tauri.conf.
            tauri::WebviewWindowBuilder::new(
                app,
                "main",
                tauri::WebviewUrl::App("index.html".into()),
            )
            .title("Langolier")
            .inner_size(1440.0, 940.0)
            .min_inner_size(800.0, 600.0)
            .background_color(tauri::window::Color(0x11, 0x12, 0x11, 0xff))
            .visible(!hidden)
            .build()?;
            set_dock_icon(ICON_PNG);
            app.manage(Tray(std::sync::Mutex::new(None)));
            if let Err(e) = apply_tray(app.handle(), s.tray_icon || hidden) {
                eprintln!("Tray: {e}");
            }
            if let Err(e) = apply_autostart(app.handle(), s.launch_at_login) {
                eprintln!("Launch at login: {e}");
            }
            crate::engine::set_cache_dir(crate::bundle::gguf_cache_dir(&db));
            crate::engine::set_idle_secs(s.engine_idle_minutes * 60);
            if s.whisper_model.is_empty() {
                s.whisper_model = db.root.join("models/ggml-base.bin").display().to_string();
                db.set_settings(&s).map_err(std::io::Error::other)?;
            }
            app.manage(AppState {
                db: db.clone(),
                cancel: Arc::new(AtomicBool::new(false)),
                generation: tokio::sync::Mutex::new(()),
            });
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(crate::ingest::worker(db.clone(), move || {
                let _ = handle.emit("library-updated", ());
            }));
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(crate::watch::worker(db, move || {
                let _ = handle.emit("library-updated", ());
            }));
            if let Err(e) = register_shortcut(app.handle(), &s.shortcut) {
                eprintln!("Palette : {e}");
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            save_settings,
            health,
            import_files,
            import_text,
            import_url,
            retry_document,
            reindex_all,
            reindex_documents,
            set_ingestion_paused,
            backup_database,
            delete_document,
            delete_documents,
            document_chunks,
            update_chunk,
            polish_chunk,
            polish_document,
            delete_chunk,
            replace_in_document,
            replace_in_chunks,
            new_conversation,
            delete_conversation,
            read_image_data_url,
            embed_models,
            api_keys,
            save_api_key,
            delete_api_key,
            use_api_key,
            telegram_check,
            save_assistant,
            delete_assistant,
            export_assistant,
            set_active_assistant,
            add_watch,
            update_watch,
            delete_watch,
            scan_watch,
            open_palette,
            hide_palette,
            pause_shortcut,
            messages,
            feedback,
            cancel_chat,
            chat,
            search_sources,
            add_evaluation,
            delete_evaluation,
            run_evaluations,
            export_data,
            pull_model
        ])
        .build(context())
        .expect("Could not start Langolier")
        .run(|app, event| {
            // Dock click while the window is hidden.
            #[cfg(target_os = "macos")]
            if let tauri::RunEvent::Reopen {
                has_visible_windows,
                ..
            } = event
            {
                if !has_visible_windows {
                    if let Some(w) = app.get_webview_window("main") {
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
            #[cfg(not(target_os = "macos"))]
            let _ = (app, event);
        });
}
/// Queues a selection of sources again.
#[tauri::command]
fn set_ingestion_paused(state: State<AppState>, paused: bool) -> Res<()> {
    let mut s = state.db.settings()?;
    s.ingestion_paused = paused;
    state.db.set_settings(&s)
}
#[tauri::command]
fn reindex_documents(state: State<AppState>, ids: Vec<String>) -> Res<usize> {
    if ids.is_empty() || ids.len() > 20000 {
        return Err("Selection empty or too large.".into());
    }
    let mut c = state.db.conn()?;
    let tx = c.transaction().map_err(err)?;
    let mut queued = 0usize;
    for id in &ids {
        queued += tx
            .execute(
                "UPDATE documents SET status='queued',stage='Reindex requested',error=NULL WHERE id=?1 AND status!='processing'",
                [id],
            )
            .map_err(err)?;
    }
    tx.commit().map_err(err)?;
    Ok(queued)
}
#[tauri::command]
fn reindex_all(state: State<AppState>) -> Res<usize> {
    state.db.conn()?.execute("UPDATE documents SET status='queued',stage='Reindex requested',error=NULL WHERE status!='processing'", []).map_err(err)
}
#[tauri::command]
fn backup_database(state: State<AppState>, path: String) -> Res<()> {
    state
        .db
        .conn()?
        .execute("VACUUM INTO ?1", [path])
        .map_err(err)?;
    Ok(())
}

/// Dock icon set at run time: a bare binary has none, and a chatbot .app
/// deserves its avatar. PNG or JPEG bytes.
#[cfg(target_os = "macos")]
pub fn set_dock_icon(bytes: &[u8]) {
    use objc2::AnyThread;
    use objc2_app_kit::{NSApplication, NSImage};
    use objc2_foundation::{MainThreadMarker, NSData};
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let data = NSData::with_bytes(bytes);
    if let Some(image) = NSImage::initWithData(NSImage::alloc(), &data) {
        let app = NSApplication::sharedApplication(mtm);
        unsafe { app.setApplicationIconImage(Some(&image)) };
    }
}
#[cfg(not(target_os = "macos"))]
pub fn set_dock_icon(_bytes: &[u8]) {}
pub const ICON_PNG: &[u8] = include_bytes!("../icons/icon.png");

#[cfg(test)]
mod integration_tests {
    use super::*;
    #[test]
    fn endpoint_policy_and_reference_bounds() {
        assert!(crate::llm::local_endpoint("http://127.0.0.1:1234/v1").is_ok());
        for url in [
            "https://example.org",
            "http://localhost.evil.org",
            "file:///tmp/model",
            "http://user:pass@localhost",
        ] {
            assert!(crate::llm::local_endpoint(url).is_err());
        }
        let v = crate::rag::citation_check("Result [1] and [5]", 2);
        assert_eq!(v["invalid"], json!([5]));
        let mut sources = vec![crate::rag::Source {
            id: 1,
            doc_id: "d".into(),
            name: "A".into(),
            text: "é".repeat(5000),
            locator: "Page 1".into(),
            score: 1.0,
        }];
        crate::rag::pack_sources(&mut sources, 500);
        assert!(sources[0].text.chars().count() < 500);
    }
    #[tokio::test]
    #[ignore = "Requires local Ollama models; optionally LANGOLIER_TEST_VIDEO and LANGOLIER_WHISPER_MODEL"]
    async fn live_local_pipeline() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::new(temp.path()).unwrap();
        let s = Settings {
            whisper_model: std::env::var("LANGOLIER_WHISPER_MODEL").unwrap_or_default(),
            ..Settings::default()
        };
        db.set_settings(&s).unwrap();
        crate::ingest::paste(&db,"Machine learning notes","# Validation\nLe surapprentissage se produit lorsque le modèle mémorise les exemples d'entraînement. La régularisation L2 pénalise les grands poids. La validation croisée évalue la généralisation sur plusieurs plis. Un jeu de test séparé reste inutilisé pendant le réglage des hyperparamètres.").unwrap();
        let docs = crate::db::json_rows(&db, "SELECT * FROM documents").unwrap();
        let d = &docs[0];
        let start = Instant::now();
        crate::ingest::process(
            &db,
            d["id"].as_str().unwrap(),
            d["source"].as_str().unwrap(),
            "md",
            "Machine learning notes",
        )
        .await
        .unwrap();
        let q = "Comment limiter le surapprentissage ?";
        let (sources, warning) = crate::rag::retrieve(&db, q, &s, "hybrid", None)
            .await
            .unwrap();
        assert!(warning.is_none(), "{warning:?}");
        assert!(!sources.is_empty());
        assert!(sources[0].text.contains("régularisation"));
        let prompt = vec![
            json!({"role":"system","content":format!("Réponds en français à partir de ces sources. Cite [1]. {}",crate::rag::context(&sources))}),
            json!({"role":"user","content":q}),
        ];
        let g = crate::llm::generate(&s, prompt, Arc::new(AtomicBool::new(false)), |_| {})
            .await
            .unwrap();
        assert!(!g.text.is_empty());
        assert!(g.tokens > 0);
        assert!(g.text.contains("[1]"));
        println!(
            "LIVE_RAG {}",
            json!({"sources":sources.len(),"tokens":g.tokens,"tps":g.tps,"first_token_ms":g.first_token_ms,"elapsed_ms":start.elapsed().as_millis(),"citation_check":crate::rag::citation_check(&g.text,sources.len())})
        );
        if let Ok(path) = std::env::var("LANGOLIER_TEST_VIDEO") {
            crate::ingest::queue(&db, vec![path.clone()]).unwrap();
            let docs =
                crate::db::json_rows(&db, "SELECT * FROM documents WHERE kind='mp4'").unwrap();
            let d = &docs[0];
            let start = Instant::now();
            crate::ingest::process(
                &db,
                d["id"].as_str().unwrap(),
                &path,
                "mp4",
                d["name"].as_str().unwrap(),
            )
            .await
            .unwrap();
            let summary=crate::db::json_rows(&db,"SELECT d.language,d.status,count(ch.id) AS chunks,count(ch.embedding) AS embedded FROM documents d JOIN chunks ch ON ch.doc_id=d.id WHERE d.kind='mp4' GROUP BY d.id").unwrap();
            assert_eq!(summary[0]["status"], "ready");
            assert!(summary[0]["chunks"].as_u64().unwrap() > 0);
            assert_eq!(summary[0]["chunks"], summary[0]["embedded"]);
            let (sources, warning) = crate::rag::retrieve(
                &db,
                "Que raconte cette vidéo à propos de la maison ?",
                &s,
                "hybrid",
                None,
            )
            .await
            .unwrap();
            assert!(warning.is_none());
            assert!(sources.iter().any(|x| x.locator.contains("-->")));
            let response=crate::llm::generate(&s,vec![json!({"role":"system","content":format!("Résume en français les faits présents dans les sources. Cite leurs numéros. {}",crate::rag::context(&sources))}),json!({"role":"user","content":"Que raconte cette vidéo à propos de la maison ?"})],Arc::new(AtomicBool::new(false)),|_|{}).await.unwrap();
            assert!(response.text.contains('['));
            println!(
                "LIVE_VIDEO {}",
                json!({"summary":summary,"elapsed_ms":start.elapsed().as_millis(),"tokens":response.tokens,"tps":response.tps,"source_locators":sources.iter().map(|s|s.locator.clone()).collect::<Vec<_>>(),"citation_check":crate::rag::citation_check(&response.text,sources.len())})
            );
        }
    }
    #[test]
    #[ignore = "Synthetic throughput benchmark, not a quality benchmark"]
    fn benchmark_50000_chunks() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::new(temp.path()).unwrap();
        let mut c = db.conn().unwrap();
        let tx = c.transaction().unwrap();
        let s = Settings::default();
        for i in 0..5000 {
            tx.execute("INSERT INTO documents(id,name,source,kind,status,created,updated) VALUES(?1,?1,?1,'md','ready',1,1)",[format!("doc-{i}")]).unwrap();
        }
        for i in 0..50000 {
            let v = (0..768)
                .map(|j| ((i * 37 + j * 13) % 997) as f32 / 997.0)
                .collect::<Vec<_>>();
            tx.execute("INSERT INTO chunks(doc_id,ordinal,text,locator,embedding,embedding_model)VALUES(?1,?2,?3,'Section 1',?4,?5)",params![format!("doc-{}",i/10),i%10,format!("Document {i} describes gradient descent, regularization and cross validation."),crate::rag::encode(&v),crate::rag::embedding_key(&s)]).unwrap();
        }
        tx.commit().unwrap();
        let q = (0..768)
            .map(|j| ((37 + j * 13) % 997) as f32 / 997.0)
            .collect::<Vec<_>>();
        let start = Instant::now();
        let r = crate::rag::search(
            &db,
            "gradient regularization",
            Some(&q),
            &s,
            "hybrid",
            s.top_k,
            None,
        )
        .unwrap();
        assert_eq!(r.len(), s.top_k);
        println!(
            "BENCHMARK {}",
            json!({"documents":5000,"chunks":50000,"dimensions":768,"retrieval_ms":start.elapsed().as_millis(),"database_bytes":db.root.join("langolier.sqlite3").metadata().unwrap().len(),"build_profile":if cfg!(debug_assertions){"debug"}else{"release"}})
        );
    }
}
