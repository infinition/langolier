//! Tauri commands for conversations, answering, and the evaluation bench.
use crate::db::{err, Res};
use crate::desktop::AppState;
use rusqlite::params;
use serde_json::{json, Value};
use std::{sync::atomic::Ordering, time::Instant};
use tauri::{Emitter, State};

#[tauri::command]
pub fn messages(state: State<AppState>, id: String) -> Res<Vec<Value>> {
    let c = state.db.conn()?;
    let mut st=c.prepare("SELECT id,role,content,sources,feedback FROM messages WHERE conversation_id=?1 ORDER BY rowid").map_err(err)?;
    let rows=st.query_map([id],|r|{let sources:String=r.get(3)?;Ok(json!({"id":r.get::<_,String>(0)?,"role":r.get::<_,String>(1)?,"content":r.get::<_,String>(2)?,"sources":serde_json::from_str::<Value>(&sources).unwrap_or(json!([])),"feedback":r.get::<_,Option<i64>>(4)?}))}).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
    Ok(rows)
}
#[tauri::command]
pub fn feedback(state: State<AppState>, id: String, value: i64) -> Res<()> {
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
pub fn cancel_chat(state: State<AppState>) {
    state.cancel.store(true, Ordering::Relaxed);
}
#[tauri::command]
pub async fn chat(
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
pub async fn search_sources(state: State<'_, AppState>, query: String, mode: String) -> Res<Value> {
    let s = state.db.settings()?;
    let start = Instant::now();
    let (sources, warning) = crate::rag::retrieve(&state.db, &query, &s, &mode, None).await?;
    Ok(json!({"sources":sources,"warning":warning,"latency_ms":start.elapsed().as_millis()}))
}
#[tauri::command]
pub fn add_evaluation(
    state: State<AppState>,
    question: String,
    expected_document: String,
) -> Res<()> {
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
pub fn delete_evaluation(state: State<AppState>, id: String) -> Res<()> {
    state
        .db
        .conn()?
        .execute("DELETE FROM evaluations WHERE id=?1", [id])
        .map_err(err)?;
    Ok(())
}
#[tauri::command]
pub async fn run_evaluations(state: State<'_, AppState>) -> Res<Value> {
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
