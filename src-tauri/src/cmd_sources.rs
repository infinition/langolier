//! Tauri commands for sources: importing, reprocessing, and editing passages.
use crate::db::{err, now, Db, Res};
use crate::desktop::AppState;
use rusqlite::params;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{Emitter, State};

#[tauri::command]
pub async fn import_files(state: State<'_, AppState>, paths: Vec<String>) -> Res<Value> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || crate::ingest::queue(&db, paths))
        .await
        .map_err(err)?
}
#[tauri::command]
pub fn import_text(state: State<AppState>, title: String, text: String) -> Res<Value> {
    crate::ingest::paste(&state.db, &title, &text)
}
#[tauri::command]
pub fn import_url(state: State<AppState>, url: String) -> Res<Value> {
    crate::ingest::queue_url(&state.db, &url)
}
#[tauri::command]
pub fn retry_document(state: State<AppState>, id: String) -> Res<()> {
    let c = state.db.conn()?;
    let changed=c.execute("UPDATE documents SET status='queued',stage='Queued',error=NULL,updated=?2 WHERE id=?1 AND status!='processing'",params![id,now()]).map_err(err)?;
    if changed == 0 {
        return Err("Source missing or already processing".into());
    }
    Ok(())
}
#[tauri::command]
pub fn delete_document(state: State<AppState>, id: String) -> Res<()> {
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
/// Queues several sources again at once, typically every source left in error.
#[tauri::command]
pub fn retry_documents(state: State<AppState>, ids: Vec<String>) -> Res<Value> {
    if ids.is_empty() || ids.len() > 20000 {
        return Err("Selection empty or too large.".into());
    }
    let mut c = state.db.conn()?;
    let tx = c.transaction().map_err(err)?;
    let mut queued = 0usize;
    for id in &ids {
        queued+=tx.execute("UPDATE documents SET status='queued',stage='Queued',error=NULL,updated=?2 WHERE id=?1 AND status!='processing'",params![id,now()]).map_err(err)?;
    }
    tx.commit().map_err(err)?;
    Ok(json!({"queued": queued, "busy": ids.len() - queued}))
}
/// Removes several sources from the index at once.
#[tauri::command]
pub fn delete_documents(state: State<AppState>, ids: Vec<String>) -> Res<Value> {
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
pub fn document_chunks(state: State<AppState>, id: String) -> Res<Vec<Value>> {
    let c = state.db.conn()?;
    let mut st = c
        .prepare("SELECT id,text,locator FROM chunks WHERE doc_id=?1 ORDER BY ordinal LIMIT 500")
        .map_err(err)?;
    let rows=st.query_map([id],|r|Ok(json!({"id":r.get::<_,i64>(0)?,"text":r.get::<_,String>(1)?,"locator":r.get::<_,String>(2)?}))).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
    Ok(rows)
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
pub async fn update_chunk(state: State<'_, AppState>, id: i64, text: String) -> Res<Value> {
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
pub async fn polish_chunk(state: State<'_, AppState>, id: i64) -> Res<Value> {
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
pub async fn polish_document(
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
pub async fn replace_in_chunks(
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
pub fn delete_chunk(state: State<AppState>, id: i64) -> Res<()> {
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
pub async fn replace_in_document(
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
