use crate::{
    db_error,
    extraction::{extract, fold},
    intelligent::{quality, LearningPolicy},
    row,
    semantic::QueryVector,
    Kind, Memory, Result, TurnRequest, COLS,
};
use rusqlite::{params, OptionalExtension, Transaction};
use std::collections::{HashMap, HashSet};

pub(crate) fn terms(query: &str) -> Vec<String> {
    let normalized = fold(query);
    let mut result = normalized
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| s.len() > 1 && !STOP.contains(s))
        .take(24)
        .map(String::from)
        .collect::<Vec<_>>();
    for (term, alias) in [
        ("jogo", "jogos"),
        ("games", "jogos"),
        ("coffee", "cafe"),
        ("computer", "computador"),
        ("computador", "ram"),
        ("notebook", "ram"),
    ] {
        if result.iter().any(|v| v == term) {
            result.push(alias.into())
        }
    }
    result.sort();
    result.dedup();
    result
}
const STOP: &[&str] = &[
    "eu", "meu", "minha", "meus", "minhas", "voce", "que", "qual", "quais", "como", "para", "por",
    "uma", "um", "de", "da", "do", "das", "dos", "em", "no", "na", "nos", "nas", "se", "com",
    "sem", "the", "what", "which", "how", "is", "are", "do", "does", "my", "your", "and", "of",
    "to", "for", "about", "tell", "recall", "remember", "lembra", "lembre", "sobre", "memoria",
    "memorias", "please", "favor", "obrigado", "thanks",
];
pub(crate) fn recall(query: &str) -> bool {
    let q = fold(query);
    [
        "o que voce lembra",
        "quais memorias",
        "what do you remember",
        "show my memories",
    ]
    .iter()
    .any(|s| q.contains(s))
}
pub(crate) fn fts_query(query: &str) -> String {
    terms(query)
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(" OR ")
}
fn eligible(tx: &Transaction, m: &Memory, query: &str, at: i64) -> Result<bool> {
    let blocked:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memory_quality q JOIN memories n ON n.id=q.memory_id WHERE q.replaces_id=?1 AND n.state IN ('candidate','conflict') AND (q.expires_at IS NULL OR q.expires_at>?2))",params![m.input.id,at],|r|r.get(0)).map_err(db_error)?;
    if blocked {
        return Ok(false);
    }
    let meta = quality(tx, &m.input.id)?;
    // A clear correction in the current user message takes precedence even
    // before the asynchronous extractor has persisted its replacement.
    if let Some(current) = &meta {
        if extract(query).iter().any(|claim| {
            (claim.subject == current.subject
                && claim.property == current.property
                && claim.value != current.value)
                || (current.property == "ram_capacity"
                    && claim.subject == "device.unspecified"
                    && crate::coverage::old_capacity(&claim.evidence).as_ref()
                        == Some(&current.value))
                || (current.property == "ram_capacity"
                    && claim.subject == current.subject
                    && claim.property == "capacity_update")
        }) {
            return Ok(false);
        }
    }
    if let Some(current) = &meta {
        let pending:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memory_quality q JOIN memories n ON n.id=q.memory_id WHERE n.scope=?1 AND n.project=?2 AND n.state IN ('active','candidate','conflict') AND q.subject=?3 AND q.property=?4 AND q.value!=?5 AND (q.expires_at IS NULL OR q.expires_at>?6))",params![m.input.scope,m.input.project,current.subject,current.property,current.value,at],|r|r.get(0)).map_err(db_error)?;
        if pending {
            return Ok(false);
        }
        if current.property == "ram_capacity" {
            let mut q=tx.prepare("SELECT p.evidence,q.subject,q.property FROM memory_provenance p JOIN memory_quality q ON q.memory_id=p.memory_id JOIN memories n ON n.id=q.memory_id WHERE n.scope=?1 AND n.project=?2 AND n.state IN ('candidate','conflict') AND (q.subject='device.unspecified' OR q.property='capacity_update') LIMIT 501").map_err(db_error)?;
            let records = q
                .query_map(params![m.input.scope, m.input.project], |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                    ))
                })
                .map_err(db_error)?;
            for (index, record) in records.enumerate() {
                if index >= 500 {
                    return Ok(false);
                }
                let (evidence, subject, property) = record.map_err(db_error)?;
                if (subject == "device.unspecified"
                    && crate::coverage::old_capacity(&evidence).as_ref() == Some(&current.value))
                    || (subject == current.subject && property == "capacity_update")
                {
                    return Ok(false);
                }
            }
        }
    }
    if meta
        .as_ref()
        .is_some_and(|q| q.expires_at.is_some_and(|expiry| expiry <= at))
    {
        return Ok(false);
    }
    if recall(query) {
        return Ok(true);
    }
    let q = fold(query);
    if let Some(meta) = &meta {
        let free = ["livre", "free", "disponivel"]
            .iter()
            .any(|word| q.contains(word));
        if meta.temporal == "temporary" && !free && !q.contains("agora") {
            return Ok(false);
        }
        if meta.property == "ram_capacity" && free {
            return Ok(false);
        }
        let compare = ["compare", "todos", "all ", "dois computadores"]
            .iter()
            .any(|word| q.contains(word));
        if !compare && meta.subject.starts_with("device.") {
            let other = q.contains("outro") || q.contains("other");
            let work = q.contains("trabalho") || q.contains("work laptop");
            if (meta.subject == "device.other_computer") != other
                || (meta.subject == "device.work_laptop") != work
            {
                return Ok(false);
            }
        }
    }
    if m.input.kind == Kind::Preference {
        let personal = [
            "recom",
            "sugir",
            "suggest",
            "prefer",
            "gosto",
            "like",
            "cafe",
            "coffee",
            "acucar",
            "que eu",
            "meus gostos",
        ]
        .iter()
        .any(|w| q.contains(w));
        if !personal {
            return Ok(false);
        }
    }
    if m.input.kind == Kind::Hypothesis
        && ![
            "hipot", "talvez", "plano", "comprar", "purchase", "might", "intenc",
        ]
        .iter()
        .any(|w| q.contains(w))
    {
        return Ok(false);
    }
    if meta.as_ref().is_some_and(|q| q.temporal == "completed") && !any_completed_intent(&q) {
        return Ok(false);
    }
    Ok(true)
}
fn any_completed_intent(q: &str) -> bool {
    ["conclu", "complet", "histor", "status"]
        .iter()
        .any(|v| q.contains(v))
}
pub(crate) fn select(
    tx: &Transaction,
    req: &TurnRequest,
    mut lexical: Vec<Memory>,
    mut used: usize,
    semantic: Option<&QueryVector>,
    policy: &LearningPolicy,
    at: i64,
) -> Result<(Vec<Memory>, usize)> {
    if recall(&req.query) {
        let mut q=tx.prepare(&format!("SELECT {COLS} FROM memories WHERE scope=?1 AND project=?2 AND state='active' ORDER BY updated_at DESC,id LIMIT 100")).map_err(db_error)?;
        lexical = q
            .query_map(params![req.scope, req.project], row)
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
    }
    let mut candidates: HashMap<String, (Memory, f64)> = HashMap::new();
    // Zero lexical weight enables the semantic-only benchmark. If the query
    // vector is unavailable, preserve the functional lexical fallback.
    if policy.lexical_weight == 0.0 && semantic.is_some() {
        lexical.clear()
    }
    for (rank, m) in lexical.into_iter().enumerate() {
        candidates.insert(
            m.input.id.clone(),
            (m, policy.lexical_weight / (1.0 + rank as f64)),
        );
    }
    if let Some(query) = semantic.filter(|_| policy.semantic_weight > 0.0) {
        query.space.check_vector(&query.vector)?;
        let space = query.space.id()?;
        let current: Option<String> = tx
            .query_row("SELECT id FROM embedding_spaces WHERE active=1", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(db_error)?;
        if current.as_ref() == Some(&space) {
            let mut stmt=tx.prepare("SELECT m.id,m.scope,m.project,m.kind,m.state,m.content,m.fact_key,m.source,m.source_id,m.version,m.created_at,m.updated_at,v.vector_json FROM memory_vectors v JOIN memories m ON m.id=v.memory_id WHERE v.space_id=?1 AND v.memory_version=m.version AND m.state='active' AND m.scope=?2 AND m.project=?3 ORDER BY m.id LIMIT 1024").map_err(db_error)?;
            let rows = stmt
                .query_map(params![space, req.scope, req.project], |r| {
                    Ok((row(r)?, r.get::<_, String>(12)?))
                })
                .map_err(db_error)?;
            for value in rows {
                let (m, encoded) = value.map_err(db_error)?;
                let vector: Vec<f32> =
                    serde_json::from_str(&encoded).map_err(|_| "Invalid indexed vector")?;
                query.space.check_vector(&vector)?;
                let similarity = vector
                    .iter()
                    .zip(&query.vector)
                    .map(|(a, b)| (*a as f64) * (*b as f64))
                    .sum::<f64>();
                if similarity >= policy.minimum_similarity {
                    let entry = candidates.entry(m.input.id.clone()).or_insert((m, 0.0));
                    entry.1 += policy.semantic_weight * similarity;
                }
            }
        }
    }
    let mut current_claims: HashMap<String, HashSet<String>> = HashMap::new();
    for (m, _) in candidates.values() {
        if !eligible(tx, m, &req.query, at)? {
            continue;
        }
        if let Some(q) = quality(tx, &m.input.id)? {
            current_claims
                .entry(format!("{}.{}", q.subject, q.property))
                .or_default()
                .insert(q.value);
        } else if let Some(c) = extract(&m.input.content).first() {
            current_claims
                .entry(c.key())
                .or_default()
                .insert(c.value.clone());
        }
    }
    let mut ranked = candidates.into_values().collect::<Vec<_>>();
    for (m, score) in &mut ranked {
        if let Some(q) = quality(tx, &m.input.id)? {
            *score += q.priority.clamp(0, 2) as f64 * 0.01;
            if matches!(q.temporal.as_str(), "temporary" | "task") {
                let age = (at - m.updated_at).max(0) as f64;
                *score += 0.01 / (1.0 + age / 3600.0);
            }
        }
    }
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.input.id.cmp(&b.0.input.id)));
    let available = req.token_budget - used;
    let mut pref_cost = 0;
    let mut seen_content = HashSet::new();
    let mut seen_property = HashSet::new();
    let mut out = Vec::new();
    for (m, _) in ranked {
        if !eligible(tx, &m, &req.query, at)? {
            continue;
        }
        let meta = quality(tx, &m.input.id)?;
        let key = meta
            .as_ref()
            .map(|q| format!("{}.{}", q.subject, q.property))
            .or_else(|| extract(&m.input.content).first().map(|c| c.key()));
        if key.as_ref().is_some_and(|k| {
            current_claims.get(k).is_some_and(|v| v.len() > 1) || seen_property.contains(k)
        }) {
            continue;
        }
        if seen_content.contains(&fold(&m.input.content)) {
            continue;
        }
        let cost = serde_json::to_vec(&m).map_err(|_| "Invalid memory")?.len() + 32;
        if used + cost > req.token_budget
            || (m.input.kind == Kind::Preference && pref_cost + cost > available / 4)
        {
            continue;
        }
        if m.input.kind == Kind::Preference {
            pref_cost += cost
        }
        used += cost;
        seen_content.insert(fold(&m.input.content));
        if let Some(key) = key {
            seen_property.insert(key);
        }
        out.push(m);
        if out.len() >= 12 {
            break;
        }
    }
    Ok((out, used))
}
