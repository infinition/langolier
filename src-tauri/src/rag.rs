use crate::db::{err, Db, Res, Settings};
use crate::llm;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Source {
    pub id: i64,
    pub doc_id: String,
    pub name: String,
    pub text: String,
    pub locator: String,
    pub score: f64,
}
pub fn chunk(text: &str, size: usize, overlap: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if size == 0 {
        return vec![];
    }
    let mut chunks = vec![];
    let mut start = 0;
    while start < chars.len() {
        let end = (start + size).min(chars.len());
        let t: String = chars[start..end].iter().collect();
        if !t.trim().is_empty() {
            chunks.push(t.trim().to_string())
        }
        if end == chars.len() {
            break;
        }
        start = end - overlap.min(size - 1)
    }
    chunks
}
/// Unit length, so that comparing two vectors is a plain dot product.
pub fn unit(v: &[f32]) -> Vec<f32> {
    let norm = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        v.iter().map(|x| x / norm).collect()
    } else {
        v.to_vec()
    }
}
/// Vectors go to storage normalized: the scan then skips both norms and the
/// square root, on every stored vector, on every question.
pub fn encode(v: &[f32]) -> Vec<u8> {
    unit(v).iter().flat_map(|x| x.to_le_bytes()).collect()
}
/// Cosine between a unit query and a stored unit vector. Kept in f32 so the
/// loop stays vectorizable; the accumulator is wide enough for 4096 dimensions.
fn cosine_blob(q: &[f32], b: &[u8]) -> f64 {
    if b.len() != q.len() * 4 {
        return -1.0;
    }
    let mut dot = 0.0f32;
    for (x, raw) in q.iter().zip(b.as_chunks::<4>().0) {
        dot += x * f32::from_le_bytes(*raw);
    }
    dot as f64
}
/// How many passages retrieval returns.
pub fn candidate_limit(s: &Settings) -> usize {
    if s.rerank_enabled {
        s.rerank_candidates.max(s.top_k)
    } else {
        s.top_k
    }
}
pub fn search(
    db: &Db,
    query: &str,
    vector: Option<&[f32]>,
    s: &Settings,
    mode: &str,
    limit: usize,
    scope: Option<&[String]>,
) -> Res<Vec<Source>> {
    let c = db.conn()?;
    let mut ranks: HashMap<i64, f64> = HashMap::new();
    let scope_json = match scope {
        Some(ids) => serde_json::to_string(ids).map_err(err)?,
        None => "null".into(),
    };
    let in_scope = "(?2='null' OR d.id IN (SELECT value FROM json_each(?2)))";
    if mode != "semantic" {
        let words = if mode == "exact" {
            let phrase = query.replace('"', " ").trim().to_string();
            if phrase.is_empty() {
                String::new()
            } else {
                format!("\"{phrase}\"")
            }
        } else {
            query
                .split(|c: char| !c.is_alphanumeric())
                .filter(|s| s.chars().count() > 1)
                .take(32)
                .map(|w| format!("\"{}\"", w.replace('"', "")))
                .collect::<Vec<_>>()
                .join(" OR ")
        };
        if !words.is_empty() {
            // FTS5 bm25() is negative; the threshold applies to its opposite.
            let mut st=c.prepare(&format!("SELECT f.rowid,-bm25(chunks_fts) FROM chunks_fts f JOIN chunks ch ON ch.id=f.rowid JOIN documents d ON d.id=ch.doc_id WHERE chunks_fts MATCH ?1 AND d.status='ready' AND {in_scope} ORDER BY bm25(chunks_fts) LIMIT 48")).map_err(err)?;
            let rows = st
                .query_map(params![words, scope_json], |r| {
                    Ok((r.get::<_, i64>(0)?, r.get::<_, f64>(1)?))
                })
                .map_err(err)?;
            let floor = s.min_lexical_score.max(0.0) as f64;
            for (i, row) in rows.enumerate() {
                let (id, score) = row.map_err(err)?;
                if floor > 0.0 && score < floor {
                    break;
                }
                *ranks.entry(id).or_default() += 1.0 / (60.0 + i as f64 + 1.0)
            }
        }
    }
    if mode != "lexical" && mode != "exact" {
        if let Some(q) = vector {
            let q = &unit(q)[..];
            let mut st=c.prepare(&format!("SELECT ch.id,ch.embedding FROM chunks ch JOIN documents d ON d.id=ch.doc_id WHERE ch.embedding_model=?1 AND ch.embedding IS NOT NULL AND d.status='ready' AND {in_scope}")).map_err(err)?;
            let mut rows = st
                .query(params![embedding_key(s), scope_json])
                .map_err(err)?;
            let mut best: Vec<(i64, f64)> = Vec::with_capacity(49);
            while let Some(r) = rows.next().map_err(err)? {
                let id: i64 = r.get(0).map_err(err)?;
                // Borrowed straight from the page cache: copying every vector
                // into a Vec costs more than the arithmetic that follows.
                let b = r.get_ref(1).map_err(err)?.as_blob().map_err(err)?;
                let score = cosine_blob(q, b);
                if score > s.min_dense_score.clamp(-1.0, 1.0) as f64 {
                    best.push((id, score));
                    best.sort_unstable_by(|a, b| b.1.total_cmp(&a.1));
                    best.truncate(48)
                }
            }
            for (i, (id, _)) in best.into_iter().enumerate() {
                *ranks.entry(id).or_default() += 1.0 / (60.0 + i as f64 + 1.0)
            }
        }
    }
    let mut ranked: Vec<_> = ranks.into_iter().collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
    let mut out = vec![];
    let mut seen = HashSet::new();
    let mut perdoc: HashMap<String, usize> = HashMap::new();
    for (id, score) in ranked {
        let source=c.query_row("SELECT ch.id,ch.doc_id,d.name,ch.text,ch.locator FROM chunks ch JOIN documents d ON d.id=ch.doc_id WHERE ch.id=?1",[id],|r|Ok(Source{id:r.get(0)?,doc_id:r.get(1)?,name:r.get(2)?,text:r.get(3)?,locator:r.get(4)?,score})).map_err(err)?;
        let count = perdoc.entry(source.doc_id.clone()).or_default();
        if *count >= 3 || !seen.insert(source.text.clone()) {
            continue;
        }
        *count += 1;
        out.push(source);
        if out.len() >= limit {
            break;
        }
    }
    Ok(out)
}
pub fn embedding_key(s: &Settings) -> String {
    format!(
        "{}|{}",
        s.embedding_endpoint.trim_end_matches('/'),
        s.embedding_model
    )
}
pub async fn retrieve(
    db: &Db,
    q: &str,
    s: &Settings,
    mode: &str,
    scope: Option<&[String]>,
) -> Res<(Vec<Source>, Option<String>)> {
    let limit = candidate_limit(s);
    if mode == "exact" {
        let mut found = search(db, q, None, s, mode, limit.max(s.top_k), scope)?;
        found.truncate(limit.max(s.top_k));
        return Ok((found, None));
    }
    if mode == "lexical" {
        return rerank(s, q, search(db, q, None, s, mode, limit, scope)?, None).await;
    }
    let (total, compatible): (i64, i64) = db.conn()?.query_row(
        "SELECT count(*),coalesce(sum(ch.embedding_model=?1 AND ch.embedding IS NOT NULL),0) FROM chunks ch JOIN documents d ON d.id=ch.doc_id WHERE d.status='ready'",
        [embedding_key(s)], |r| Ok((r.get(0)?,r.get(1)?))
    ).map_err(err)?;
    if total == 0 {
        return Ok((vec![], None));
    }
    if compatible == 0 {
        let message =
            "No compatible vector. Reindex the sources with the configured embedding model.";
        if mode == "semantic" {
            return Err(message.into());
        }
        return rerank(
            s,
            q,
            search(db, q, None, s, "lexical", limit, scope)?,
            Some(message.into()),
        )
        .await;
    }
    let coverage_warning = (compatible < total).then(||format!("Partial semantic index: {compatible}/{total} compatible passages. Reindex the remaining sources."));
    match llm::embeddings(s, &[q.to_string()]).await {
        Ok(v) => {
            rerank(
                s,
                q,
                search(db, q, Some(&v[0]), s, mode, limit, scope)?,
                coverage_warning,
            )
            .await
        }
        Err(e) => {
            if mode == "semantic" {
                Err(e)
            } else {
                rerank(
                    s,
                    q,
                    search(db, q, None, s, "lexical", limit, scope)?,
                    Some(format!("Recherche lexicale seule : {e}")),
                )
                .await
            }
        }
    }
}
fn join_warning(a: Option<String>, b: Option<String>) -> Option<String> {
    match (a, b) {
        (Some(a), Some(b)) => Some(format!("{a} {b}")),
        (a, b) => a.or(b),
    }
}
/// Second pass by the judge model, then the abstention threshold.
async fn rerank(
    s: &Settings,
    q: &str,
    mut sources: Vec<Source>,
    warning: Option<String>,
) -> Res<(Vec<Source>, Option<String>)> {
    if !s.rerank_enabled || sources.is_empty() {
        sources.truncate(s.top_k);
        return Ok((sources, warning));
    }
    let candidates = sources
        .iter()
        .map(|x| (x.id, format!("{} · {}\n{}", x.name, x.locator, x.text)))
        .collect::<Vec<_>>();
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    match llm::rerank(s, q, &candidates, cancel).await {
        Ok(scores) => {
            let by_id: HashMap<i64, f32> = scores.into_iter().collect();
            for x in sources.iter_mut() {
                x.score = *by_id.get(&x.id).unwrap_or(&0.0) as f64;
            }
            let floor = s.rerank_threshold.clamp(0.0, 1.0) as f64;
            let best = sources.iter().map(|x| x.score).fold(0.0, f64::max);
            sources.retain(|x| x.score >= floor);
            sources.sort_by(|a, b| b.score.total_cmp(&a.score));
            sources.truncate(s.top_k);
            let note = sources.is_empty().then(|| {
                format!(
                    "No passage clears the relevance threshold ({:.0}%; best candidate {:.0}%).",
                    floor * 100.0,
                    best * 100.0
                )
            });
            Ok((sources, join_warning(warning, note)))
        }
        Err(e) => {
            sources.truncate(s.top_k);
            Ok((
                sources,
                join_warning(
                    warning,
                    Some(format!(
                        "Juge de pertinence indisponible, tri par fusion seule : {e}"
                    )),
                ),
            ))
        }
    }
}
pub fn mask_sources(sources: &[Source]) -> Vec<Source> {
    sources
        .iter()
        .enumerate()
        .map(|(i, s)| Source {
            name: format!("Source {}", i + 1),
            ..s.clone()
        })
        .collect()
}
pub fn context(sources: &[Source]) -> String {
    sources
        .iter()
        .enumerate()
        .map(|(i, s)| format!("[{}] {}  ·  {}\n{}", i + 1, s.name, s.locator, s.text))
        .collect::<Vec<_>>()
        .join("\n\n")
}
pub fn history(db: &Db, id: &str) -> Res<Vec<serde_json::Value>> {
    let c = db.conn()?;
    let mut st=c.prepare("SELECT role,content FROM (SELECT role,content,created,rowid AS seq FROM messages WHERE conversation_id=?1 ORDER BY rowid DESC LIMIT 8) ORDER BY seq ASC").map_err(err)?;
    let out=st.query_map(params![id],|r|{let role:String=r.get(0)?;let content:String=r.get(1)?;Ok(serde_json::json!({"role":role,"content":content.chars().take(2400).collect::<String>()}))}).map_err(err)?.collect::<Result<Vec<_>,_>>().map_err(err)?;
    Ok(out)
}
pub fn fuse(a: &[Source], b: &[Source], limit: usize) -> Vec<Source> {
    let mut merged: HashMap<i64, Source> = HashMap::new();
    for list in [a, b] {
        for (i, s) in list.iter().enumerate() {
            let item = merged.entry(s.id).or_insert_with(|| {
                let mut c = s.clone();
                c.score = 0.0;
                c
            });
            item.score += 1.0 / (61.0 + i as f64)
        }
    }
    let mut out = merged.into_values().collect::<Vec<_>>();
    out.sort_by(|a, b| b.score.total_cmp(&a.score));
    out.truncate(limit);
    out
}
pub fn citation_check(text: &str, count: usize) -> serde_json::Value {
    let re = regex::Regex::new(r"\[(\d+)\]").unwrap();
    let refs = re
        .captures_iter(text)
        .filter_map(|c| c[1].parse::<usize>().ok())
        .collect::<Vec<_>>();
    let invalid = refs
        .iter()
        .filter(|n| **n == 0 || **n > count)
        .copied()
        .collect::<Vec<_>>();
    serde_json::json!({"references":refs.len(),"invalid":invalid,"missing":count>0&&refs.is_empty(),"note":"Reference bounds only; does not measure factual faithfulness"})
}
pub fn pack_sources(sources: &mut Vec<Source>, budget: usize) {
    let mut remaining = budget;
    sources.retain_mut(|source| {
        let overhead = source.name.chars().count() + source.locator.chars().count() + 30;
        if remaining < overhead + 150 {
            return false;
        }
        let length = source.text.chars().count().min(remaining - overhead);
        source.text = source.text.chars().take(length).collect();
        remaining = remaining.saturating_sub(length + overhead);
        true
    });
}

#[cfg(test)]
mod bench {
    use super::*;
    /// Times a dense scan over the caller's real library.
    /// LANGOLIER_BENCH_DB=/path/to/langolier.sqlite3 cargo test dense_scan -- --ignored --nocapture
    #[test]
    #[ignore = "Needs LANGOLIER_BENCH_DB pointing at a populated library"]
    fn dense_scan() {
        let Ok(path) = std::env::var("LANGOLIER_BENCH_DB") else {
            return;
        };
        let root = std::path::PathBuf::from(&path);
        let root = root.parent().unwrap();
        let db = Db::new(root).unwrap();
        let mut s = Settings::default();
        s.embedding_endpoint = "http://127.0.0.1:11434".into();
        s.embedding_model = "embeddinggemma".into();
        s.min_dense_score = -1.0;
        let dims = {
            let c = db.conn().unwrap();
            c.query_row(
                "SELECT length(embedding)/4 FROM chunks WHERE embedding IS NOT NULL LIMIT 1",
                [],
                |r| r.get::<_, usize>(0),
            )
            .unwrap()
        };
        let q: Vec<f32> = (0..dims).map(|i| ((i % 17) as f32) / 17.0 - 0.5).collect();
        for pass in 1..=3 {
            // A fresh Db owns an empty pool: this is what every search used to pay.
            let cold = Db::new(root).unwrap();
            let start = std::time::Instant::now();
            search(&cold, "", Some(&q), &s, "semantic", 6, None).unwrap();
            let fresh = start.elapsed();
            let start = std::time::Instant::now();
            let found = search(&db, "", Some(&q), &s, "semantic", 6, None).unwrap();
            println!(
                "pass {pass}: fresh connection {} ms, pooled {} ms, {} passages, {dims} dims",
                fresh.as_millis(),
                start.elapsed().as_millis(),
                found.len()
            );
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_chunking() {
        let x = "é猫🙂bonjour".repeat(1000);
        let c = chunk(&x, 1200, 180);
        assert!(c.len() > 5);
        assert!(c.iter().all(|s| s.chars().count() <= 1200));
        assert_eq!(chunk("", 5, 2).len(), 0)
    }
    #[test]
    fn vector_dimensions() {
        assert!((cosine_blob(&unit(&[1.0, 0.0]), &encode(&[3.0, 0.0])) - 1.0).abs() < 1e-6);
        assert!((cosine_blob(&unit(&[1.0, 1.0]), &encode(&[2.0, 0.0])) - 0.707).abs() < 1e-3);
        assert!(cosine_blob(&unit(&[1.0, 0.0]), &encode(&[-1.0, 0.0])) < -0.99);
        assert_eq!(cosine_blob(&[1.0], &encode(&[1.0, 2.0])), -1.0);
        assert_eq!(unit(&[0.0, 0.0]), vec![0.0, 0.0])
    }
    /// Dense ranking must follow the angle between vectors, whatever length
    /// the embedding model hands back, and whatever length was stored.
    #[test]
    fn dense_ranking_ignores_vector_length() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Db::new(tmp.path()).unwrap();
        let mut s = Settings::default();
        s.embedding_endpoint = "e".into();
        s.embedding_model = "m".into();
        s.min_dense_score = -1.0;
        let key = embedding_key(&s);
        let c = db.conn().unwrap();
        c.execute("INSERT INTO documents(id,name,source,kind,status,created,updated)VALUES('a','A','x','md','ready',1,1)",[]).unwrap();
        c.execute("INSERT INTO documents(id,name,source,kind,status,created,updated)VALUES('b','B','x','md','ready',1,1)",[]).unwrap();
        // The near match is stored 50 times longer than the far one.
        c.execute("INSERT INTO chunks(doc_id,ordinal,text,locator,embedding,embedding_model)VALUES('a',0,'near','p',?1,?2)",params![encode(&[50.0,1.0]),&key]).unwrap();
        c.execute("INSERT INTO chunks(doc_id,ordinal,text,locator,embedding,embedding_model)VALUES('b',0,'far','p',?1,?2)",params![encode(&[0.02,1.0]),&key]).unwrap();
        let q = [3.0f32, 0.0];
        let r = search(&db, "", Some(&q), &s, "semantic", 5, None).unwrap();
        assert_eq!(r.len(), 2);
        assert_eq!(r[0].doc_id, "a", "the closest angle must win");
        // A threshold above the far vector's cosine drops it.
        s.min_dense_score = 0.5;
        let r = search(&db, "", Some(&q), &s, "semantic", 5, None).unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].doc_id, "a")
    }
    #[test]
    fn fts_cascade_and_rank() {
        let tmp = tempfile::tempdir().unwrap();
        let db = Db::new(tmp.path()).unwrap();
        let c = db.conn().unwrap();
        c.execute("INSERT INTO documents(id,name,source,kind,status,created,updated)VALUES('a','Cours ML','x','md','ready',1,1)",[]).unwrap();
        c.execute("INSERT INTO chunks(doc_id,ordinal,text,locator)VALUES('a',0,'La régularisation limite le surapprentissage','page 1')",[]).unwrap();
        let s = Settings::default();
        let r = search(&db, "régularisation", None, &s, "lexical", s.top_k, None).unwrap();
        assert_eq!(r[0].doc_id, "a");
        assert_eq!(
            search(
                &db,
                "limite le surapprentissage",
                None,
                &s,
                "exact",
                5,
                None
            )
            .unwrap()
            .len(),
            1
        );
        assert!(search(
            &db,
            "régularisation",
            None,
            &s,
            "lexical",
            5,
            Some(&["zzz".to_string()])
        )
        .unwrap()
        .is_empty());
        assert_eq!(
            search(
                &db,
                "régularisation",
                None,
                &s,
                "lexical",
                5,
                Some(&["a".to_string()])
            )
            .unwrap()
            .len(),
            1
        );
        assert!(
            search(&db, "surapprentissage limite", None, &s, "exact", 5, None)
                .unwrap()
                .is_empty()
        );
        assert!(search(&db, "\"", None, &s, "exact", 5, None).is_ok());
        assert!(search(&db, "\" OR *", None, &s, "lexical", s.top_k, None).is_ok());
        c.execute("DELETE FROM documents WHERE id='a'", []).unwrap();
        assert!(
            search(&db, "régularisation", None, &s, "lexical", s.top_k, None)
                .unwrap()
                .is_empty()
        )
    }
}
