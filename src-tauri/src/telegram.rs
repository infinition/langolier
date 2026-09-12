//! Telegram bridge: long polling, no port to open.
use crate::{
    assistant::{self, Assistant},
    db::{err, now, Db, Res},
    pipeline,
};
use serde_json::{json, Value};
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;

pub fn schema() -> &'static str {
    "CREATE TABLE IF NOT EXISTS telegram_chats(chat_id INTEGER PRIMARY KEY,conversation_id TEXT NOT NULL,assistant_id TEXT NOT NULL,updated INTEGER NOT NULL);
     CREATE TABLE IF NOT EXISTS telegram_state(assistant_id TEXT PRIMARY KEY,offset INTEGER NOT NULL DEFAULT 0);"
}
fn api(token: &str, method: &str) -> String {
    format!("https://api.telegram.org/bot{token}/{method}")
}
fn client(timeout: u64) -> Res<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(timeout))
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(err)
}
/// getMe validates the token and returns the bot name.
pub async fn check(token: &str) -> Res<Value> {
    let token = token.trim();
    if token.is_empty() || !token.contains(':') {
        return Err("Invalid bot token: it looks like 123456789:AAH…".into());
    }
    let v: Value = client(15)?
        .get(api(token, "getMe"))
        .send()
        .await
        .map_err(|e| format!("Telegram injoignable : {e}"))?
        .json()
        .await
        .map_err(err)?;
    if v["ok"] != true {
        return Err(format!(
            "Telegram refuse ce jeton : {}",
            v["description"].as_str().unwrap_or("?")
        ));
    }
    Ok(
        json!({"username": v["result"]["username"], "name": v["result"]["first_name"], "id": v["result"]["id"]}),
    )
}
fn allowed(a: &Assistant, from: &Value) -> bool {
    let list: Vec<String> = a
        .telegram_allowed
        .split(|c: char| c == ',' || c.is_whitespace())
        .map(|x| x.trim().trim_start_matches('@').to_lowercase())
        .filter(|x| !x.is_empty())
        .collect();
    if list.is_empty() {
        return true;
    }
    let id = from["id"]
        .as_i64()
        .map(|i| i.to_string())
        .unwrap_or_default();
    let user = from["username"].as_str().unwrap_or("").to_lowercase();
    list.iter()
        .any(|x| *x == id || (!user.is_empty() && *x == user))
}
/// Telegram text: our light markup becomes plain text.
fn plain(s: &str) -> String {
    s.replace("**", "").replace("```", "\n").trim().to_string()
}
async fn send(token: &str, chat_id: i64, text: &str) {
    let Ok(c) = client(30) else { return };
    // Telegram caps messages at 4096 characters.
    let chars: Vec<char> = text.chars().collect();
    for part in chars.chunks(4000) {
        let body: String = part.iter().collect();
        let _ = c
            .post(api(token, "sendMessage"))
            .json(&json!({"chat_id": chat_id, "text": body, "disable_web_page_preview": true}))
            .send()
            .await;
    }
}
async fn typing(token: &str, chat_id: i64) {
    if let Ok(c) = client(10) {
        let _ = c
            .post(api(token, "sendChatAction"))
            .json(&json!({"chat_id": chat_id, "action": "typing"}))
            .send()
            .await;
    }
}
fn conversation_for(db: &Db, chat_id: i64, a: &Assistant, fresh: bool) -> Res<String> {
    let c = db.conn()?;
    if !fresh {
        if let Ok(id) = c.query_row(
            "SELECT conversation_id FROM telegram_chats WHERE chat_id=?1",
            [chat_id],
            |r| r.get::<_, String>(0),
        ) {
            let alive: i64 = c
                .query_row(
                    "SELECT count(*) FROM conversations WHERE id=?1",
                    [&id],
                    |r| r.get(0),
                )
                .map_err(err)?;
            if alive > 0 {
                return Ok(id);
            }
        }
    }
    drop(c);
    let id = pipeline::new_conversation(db, Some(&a.id))?;
    db.conn()?
        .execute(
            "INSERT INTO telegram_chats(chat_id,conversation_id,assistant_id,updated) VALUES(?1,?2,?3,?4) ON CONFLICT(chat_id) DO UPDATE SET conversation_id=excluded.conversation_id,assistant_id=excluded.assistant_id,updated=excluded.updated",
            rusqlite::params![chat_id, id, a.id, now()],
        )
        .map_err(err)?;
    Ok(id)
}
/// Polling loop.
pub async fn poll(db: Db, generation: Arc<tokio::sync::Mutex<()>>) {
    let mut backoff = 2u64;
    loop {
        // The profile and its token are reread every pass, so updates need no restart.
        let (a, s) = match db
            .settings()
            .and_then(|s| assistant::get(&db, &s.active_assistant).map(|a| (a, s)))
        {
            Ok((Some(a), s)) if !a.telegram_token.trim().is_empty() => (a, s),
            _ => {
                tokio::time::sleep(Duration::from_secs(20)).await;
                continue;
            }
        };
        let token = a.telegram_token.trim().to_string();
        let offset: i64 = db
            .conn()
            .and_then(|c| {
                c.query_row(
                    "SELECT offset FROM telegram_state WHERE assistant_id=?1",
                    [&a.id],
                    |r| r.get(0),
                )
                .map_err(err)
            })
            .unwrap_or(0);
        let updates: Result<Value, String> = async {
            let c = client(45)?;
            let r = c
                .get(api(&token, "getUpdates"))
                .query(&[
                    ("timeout", "30"),
                    ("offset", &offset.to_string()),
                    ("allowed_updates", "[\"message\"]"),
                ])
                .send()
                .await
                .map_err(err)?
                .error_for_status()
                .map_err(err)?;
            r.json::<Value>().await.map_err(err)
        }
        .await;
        let v = match updates {
            Ok(v) => v,
            Err(e) => {
                eprintln!("Telegram : {e} — nouvel essai dans {backoff} s");
                tokio::time::sleep(Duration::from_secs(backoff)).await;
                backoff = (backoff * 2).min(120);
                continue;
            }
        };
        backoff = 2;
        if v["ok"] != true {
            eprintln!(
                "Telegram : {}",
                v["description"].as_str().unwrap_or("invalid response")
            );
            tokio::time::sleep(Duration::from_secs(30)).await;
            continue;
        }
        for u in v["result"].as_array().cloned().unwrap_or_default() {
            let update_id = u["update_id"].as_i64().unwrap_or(0);
            let _ = db.conn().and_then(|c| {
                c.execute("INSERT INTO telegram_state(assistant_id,offset) VALUES(?1,?2) ON CONFLICT(assistant_id) DO UPDATE SET offset=excluded.offset", rusqlite::params![a.id, update_id + 1]).map_err(err)
            });
            let m = &u["message"];
            let Some(chat_id) = m["chat"]["id"].as_i64() else {
                continue;
            };
            let text = m["text"].as_str().unwrap_or("").trim().to_string();
            if text.is_empty() {
                continue;
            }
            if !allowed(&a, &m["from"]) {
                send(&token, chat_id, "This assistant is private.").await;
                continue;
            }
            if text == "/start" || text == "/new" || text == "/nouveau" {
                let _ = conversation_for(&db, chat_id, &a, true);
                let hello = if a.welcome.trim().is_empty() {
                    format!("Bonjour, je suis {}. Posez-moi votre question.", a.name)
                } else {
                    a.welcome.clone()
                };
                send(&token, chat_id, &hello).await;
                continue;
            }
            let conv = match conversation_for(&db, chat_id, &a, false) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Telegram : {e}");
                    continue;
                }
            };
            typing(&token, chat_id).await;
            let effective = match assistant::effective(&s, Some(&a)) {
                Ok(e) => e,
                Err(e) => {
                    send(&token, chat_id, &format!("Configuration invalide : {e}")).await;
                    continue;
                }
            };
            let token2 = token.clone();
            let typing_task = tokio::spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(4)).await;
                    typing(&token2, chat_id).await;
                }
            });
            let reply = {
                let _lock = generation.lock().await;
                pipeline::answer(
                    &db,
                    &effective,
                    Some(&a),
                    &conv,
                    &text,
                    "hybrid",
                    false,
                    Arc::new(AtomicBool::new(false)),
                    &|_, _| {},
                )
                .await
            };
            typing_task.abort();
            match reply {
                Ok(r) => {
                    let mut out = plain(r["content"].as_str().unwrap_or(""));
                    if a.show_sources {
                        if let Some(list) = r["sources"].as_array().filter(|l| !l.is_empty()) {
                            let refs: Vec<String> = list
                                .iter()
                                .take(4)
                                .enumerate()
                                .map(|(i, s)| {
                                    format!("[{}] {}", i + 1, s["name"].as_str().unwrap_or(""))
                                })
                                .collect();
                            out.push_str(&format!("\n\n{}", refs.join(" · ")));
                        }
                    }
                    send(&token, chat_id, &out).await;
                }
                Err(e) => {
                    send(
                        &token,
                        chat_id,
                        &format!("Sorry, something went wrong: {e}"),
                    )
                    .await
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn whitelist_and_plain() {
        let mut a = Assistant {
            telegram_allowed: "@Moi, 42".into(),
            ..Default::default()
        };
        assert!(allowed(&a, &json!({"id": 42, "username": "x"})));
        assert!(allowed(&a, &json!({"id": 7, "username": "moi"})));
        assert!(!allowed(&a, &json!({"id": 7, "username": "autre"})));
        a.telegram_allowed.clear();
        assert!(allowed(&a, &json!({"id": 7})));
        assert_eq!(plain("**gras** et `code`"), "gras et `code`");
    }
    #[test]
    #[ignore = "reseau : cargo test telegram -- --ignored"]
    fn bad_token_is_rejected_cleanly() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let e = rt.block_on(check("123456:AAHinvalid")).unwrap_err();
        assert!(e.contains("refuse") || e.contains("injoignable"), "{e}");
        assert!(rt.block_on(check("pas-un-jeton")).is_err());
    }
}
