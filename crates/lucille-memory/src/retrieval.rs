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
        ("rpgs", "rpg"),
        ("professions", "classes"),
        ("profissoes", "classes"),
        ("laptop", "notebook"),
        ("memory", "ram"),
        ("office", "trabalho"),
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
    "memorias", "please", "favor", "obrigado", "thanks", "in", "on", "at", "our", "has", "have",
    "am", "an", "it", "its", "would", "should", "much", "many",
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
fn communication_intent(query: &str) -> bool {
    let q = fold(query);
    let words = q
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>();
    let personal = crate::relevance::personal(query)
        || format!(" {words} ").contains(" para mim ")
        || format!(" {words} ").contains(" to me ");
    personal
        && [
            "explic",
            "explain",
            "communication style",
            "estilo de resposta",
            "formato das respostas",
            "how should you respond",
        ]
        .iter()
        .any(|w| q.contains(w))
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
        let current_claims = extract(query)
            .into_iter()
            .chain(crate::dialogue::extract(query, &m.input.project));
        if current_claims.into_iter().any(|claim| {
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
            let changed:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memory_quality q JOIN memories n ON n.id=q.memory_id WHERE n.scope=?1 AND n.project=?2 AND n.state IN ('candidate','conflict') AND q.property='equipment_change' AND (q.subject='device.unspecified' OR q.subject=?3))",params![m.input.scope,m.input.project,current.subject],|r|r.get(0)).map_err(db_error)?;
            if changed {
                return Ok(false);
            }
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
    if crate::relevance::knowledge_only(query) {
        return Ok(false);
    }
    if [
        "meu amigo",
        "meu colega",
        "minha colega",
        "my friend",
        "my colleague",
        "outra pessoa",
    ]
    .iter()
    .any(|w| fold(query).contains(w))
    {
        return Ok(false);
    }
    let q = fold(query);
    let summary = crate::dialogue::project_summary(query);
    if meta
        .as_ref()
        .is_some_and(|m| m.property == "description" && m.subject.starts_with("project."))
        && !summary
        && !["projet", "project"].iter().any(|w| q.contains(w))
    {
        return Ok(false);
    }
    if summary
        && meta
            .as_ref()
            .is_none_or(|meta| meta.subject != format!("project.{}", m.input.project))
    {
        return Ok(false);
    }
    let words = q
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
        .collect::<Vec<_>>();
    if words.windows(2).any(|p| {
        ["projeto", "project"].contains(&p[0])
            && !p[1].is_empty()
            && ![
                "uses",
                "usa",
                "database",
                "banco",
                "dados",
                "stores",
                "should",
                "has",
                "tem",
                "requer",
                "requires",
                "ficticio",
                "configuration",
                "de",
                "do",
                "da",
                "going",
                "doing",
            ]
            .contains(&p[1])
            && p[1] != fold(&m.input.project)
    }) {
        return Ok(false);
    }
    if let Some(meta) = &meta {
        if meta.property == "language"
            && !summary
            && ![
                "project",
                "projeto",
                "language",
                "linguagem",
                "stack",
                "tecnologia",
                "escolhi",
                "decidi",
                "chosen",
            ]
            .iter()
            .any(|p| q.contains(p))
        {
            return Ok(false);
        }
        if ["ram_capacity", "free_ram"].contains(&meta.property.as_str())
            && [
                "ssd",
                "storage",
                "disco",
                "armazenamento",
                "disk",
                "gpu",
                "vram",
                "cpu",
                "processor",
                "processador",
                "graphics",
                "placa de video",
            ]
            .iter()
            .any(|w| q.contains(w))
            && !words.contains(&"ram")
        {
            return Ok(false);
        }
        if meta.property == "database"
            && !summary
            && ![
                "database",
                "banco",
                "dados",
                "data",
                "sqlite",
                "postgres",
                "mysql",
                "engine",
                "relational",
                "relacional",
                "armaz",
                "persist",
                "store",
            ]
            .iter()
            .any(|w| q.contains(w))
        {
            return Ok(false);
        }
        if matches!(m.input.kind, Kind::Goal | Kind::Task)
            && !crate::relevance::personal(&q)
            && ![
                "goal",
                "objetiv",
                "meta",
                "milestone",
                "pending",
                "due",
                "amanha",
                "tomorrow",
                "hoje",
                "today",
                "conclu",
                "complet",
                "finish",
                "status",
                "histor",
            ]
            .iter()
            .any(|w| q.contains(w))
        {
            return Ok(false);
        }
        let free = ["livre", "free", "disponivel", "available"]
            .iter()
            .any(|word| q.contains(word));
        if meta.temporal == "temporary" && !free && !q.contains("agora") && !q.contains("now") {
            return Ok(false);
        }
        if meta.property == "ram_capacity" && free {
            return Ok(false);
        }
        if meta.property == "response_language"
            && ["learn", "aprend", "estud"].iter().any(|w| q.contains(w))
        {
            return Ok(false);
        }
        let compare = ["compare", "todos", "all ", "dois computadores"]
            .iter()
            .any(|word| q.contains(word));
        if !compare && meta.subject.starts_with("device.") {
            let other = q.contains("outro")
                || q.contains("other")
                || q.contains("secondary")
                || q.contains("secundario");
            let work = [
                "trabalho",
                "work laptop",
                "office notebook",
                "office laptop",
                "empresa",
                "corporativo",
            ]
            .iter()
            .any(|w| q.contains(w));
            let primary = crate::coverage::device(&q) == Some("device.primary_computer")
                || ["main desktop", "primary", "principal"]
                    .iter()
                    .any(|w| q.contains(w));
            if meta.property == "ram_capacity" && !work && !other && !primary {
                let count:i64=tx.query_row("SELECT count(DISTINCT q.subject) FROM memory_quality q JOIN memories m ON m.id=q.memory_id WHERE m.scope=?1 AND m.project=?2 AND m.state='active' AND q.property='ram_capacity' AND (q.expires_at IS NULL OR q.expires_at>?3)",params![m.input.scope,m.input.project,at],|r|r.get(0)).map_err(db_error)?;
                if count > 1 {
                    return Ok(false);
                }
            }
            if (meta.subject == "device.other_computer") != other
                || (meta.subject == "device.work_laptop") != work
            {
                return Ok(false);
            }
        }
    }
    if m.input.kind == Kind::Preference {
        let personal = crate::relevance::preference_intent(query)
            || (communication_intent(query)
                && meta
                    .as_ref()
                    .is_some_and(|m| m.property == "communication_style"));
        if !personal {
            return Ok(false);
        }
    }
    if m.input.kind == Kind::Hypothesis
        && ![
            "hipot", "talvez", "plano", "comprar", "purchase", "might", "intenc", "buy", "plan",
            "consider",
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
    ["conclu", "complet", "histor", "status", "finish"]
        .iter()
        .any(|v| q.contains(v))
}

impl crate::Store {
    /// QA diagnostics reuse the actual selector and gates. No cross-scope content
    /// is returned. Raw rank, nomination and injected context remain distinct.
    pub fn explain_context(
        &mut self,
        req: TurnRequest,
        vector: Option<&QueryVector>,
    ) -> Result<serde_json::Value> {
        self.explain_context_at(req, vector, crate::now())
    }
    /// Uses the selector's explicit logical clock for isolated expiry evaluation.
    pub fn explain_context_at(
        &mut self,
        req: TurnRequest,
        vector: Option<&QueryVector>,
        at: i64,
    ) -> Result<serde_json::Value> {
        let context = self.context_at(req.clone(), vector, at)?;
        let policy = self.learning_policy()?;
        let tx = self.db.transaction().map_err(db_error)?;
        let mut lexical = HashMap::new();
        let fts = fts_query(&req.query);
        if !fts.is_empty() {
            let mut q=tx.prepare("SELECT m.id,bm25(memories_fts) FROM memories_fts JOIN memories m ON m.rowid=memories_fts.rowid WHERE memories_fts MATCH ?1 AND m.scope=?2 AND m.project=?3 AND m.state='active' ORDER BY bm25(memories_fts),m.id LIMIT 100").map_err(db_error)?;
            for (rank, r) in q
                .query_map(params![fts, req.scope, req.project], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
                })
                .map_err(db_error)?
                .enumerate()
            {
                let (id, score) = r.map_err(db_error)?;
                lexical.insert(id, (rank + 1, score));
            }
        }
        let mut q = tx
            .prepare(&format!(
                "SELECT {COLS} FROM memories WHERE scope=?1 AND project=?2 ORDER BY id LIMIT 1024"
            ))
            .map_err(db_error)?;
        let memories = q
            .query_map(params![req.scope, req.project], row)
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        let space = vector.map(|v| v.space.id()).transpose()?;
        let mut trace = vec![];
        for m in memories {
            let meta = quality(&tx, &m.input.id)?;
            let stored:Option<(i64,String,String)>=tx.query_row("SELECT memory_version,space_id,vector_json FROM memory_vectors WHERE memory_id=?1",[&m.input.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(db_error)?;
            let mut similarity = None;
            if let (Some(v), Some((version, id, encoded))) = (vector, stored.as_ref()) {
                if *version == m.version && Some(id) == space.as_ref() {
                    let dv: Vec<f32> =
                        serde_json::from_str(encoded).map_err(|_| "Invalid vector")?;
                    v.space.check_vector(&dv)?;
                    similarity = Some(
                        dv.iter()
                            .zip(&v.vector)
                            .map(|(a, b)| *a as f64 * *b as f64)
                            .sum::<f64>(),
                    );
                }
            }
            let active = m.state == crate::Status::Active;
            let gate = active && eligible(&tx, &m, &req.query, at)?;
            let coverage = crate::relevance::lexical_coverage(&req.query, &m.input.content);
            let selected = context
                .references
                .iter()
                .any(|r| r.id == m.input.id && r.version == m.version);
            trace.push(serde_json::json!({"id":m.input.id,"content":m.input.content,"version":m.version,"state":m.state,"quality":meta,"scope_project_pass":true,"state_temporal_entity_intent_pass":gate,"lexical_rank_bm25":lexical.get(&m.input.id),"lexical_coverage":coverage,"lexical_gate":coverage>=crate::relevance::LEXICAL_COVERAGE,"cosine":similarity,"semantic_threshold_pass":similarity.is_some_and(|s|s>=policy.minimum_similarity),"selected":selected,"reason":if selected{"selected_after_all_filters"}else if !active{"not_current"}else if !gate{"temporal_entity_intent_or_conflict_filter"}else{"threshold_margin_rank_diversity_or_budget"}}));
        }
        let mut raw = trace
            .iter()
            .filter(|v| v["state_temporal_entity_intent_pass"] == true && v["cosine"].is_number())
            .map(|v| (v["id"].clone(), v["cosine"].as_f64().unwrap()))
            .collect::<Vec<_>>();
        raw.sort_by(|a, b| b.1.total_cmp(&a.1));
        Ok(
            serde_json::json!({"context":context,"trace":trace,"semantic_rank_before_threshold":raw,"semantic_margin":raw.first().map(|x|x.1-raw.get(1).map_or(0.,|x|x.1)),"policy":policy,"scope_filter":{"scope":req.scope,"project":req.project},"limits":{"lexical_candidates":100,"vectors":1024,"context_records_single_intent":3,"context_records_explicit_multi":12},"used_upper_bound":context.used_upper_bound}),
        )
    }
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
    let summary = crate::dialogue::project_summary(&req.query);
    let communication = communication_intent(&req.query);
    if recall(&req.query) || summary || communication {
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
    for (_rank, m) in lexical.into_iter().enumerate() {
        let coverage = crate::relevance::lexical_coverage(&req.query, &m.input.content);
        let style_match = communication
            && quality(tx, &m.input.id)?.is_some_and(|q| q.property == "communication_style");
        if !recall(&req.query)
            && !summary
            && !style_match
            && coverage < crate::relevance::LEXICAL_COVERAGE
        {
            continue;
        }
        candidates.insert(
            m.input.id.clone(),
            (
                m,
                if recall(&req.query) || summary || style_match {
                    1.0
                } else {
                    policy.lexical_weight * coverage
                },
            ),
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
            let mut semantic_ranked = Vec::new();
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
                if eligible(tx, &m, &req.query, at)? {
                    semantic_ranked.push((m, similarity));
                }
            }
            semantic_ranked
                .sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.input.id.cmp(&b.0.input.id)));
            let multiple = recall(&req.query)
                || summary
                || ["compare", "todos", "all ", "dois computadores"]
                    .iter()
                    .any(|w| fold(&req.query).contains(w));
            let margin = semantic_ranked
                .first()
                .map_or(0., |x| x.1 - semantic_ranked.get(1).map_or(0., |s| s.1));
            for (m, similarity) in semantic_ranked
                .into_iter()
                .take(if multiple { 12 } else { 1 })
            {
                if similarity >= policy.minimum_similarity
                    && (multiple || margin >= crate::relevance::SEMANTIC_MARGIN)
                {
                    let entry = candidates.entry(m.input.id.clone()).or_insert((m, 0.));
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
    let mut covered_terms = HashSet::new();
    let mut seen_kinds = HashSet::new();
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
        let multiple = recall(&req.query)
            || summary
            || ["compare", "todos", "all ", "dois computadores"]
                .iter()
                .any(|w| fold(&req.query).contains(w));
        let doc = crate::relevance::tokens(&m.input.content);
        let matched = crate::relevance::tokens(&req.query)
            .into_iter()
            .filter(|v| doc.contains(v))
            .collect::<Vec<_>>();
        let kind = format!("{:?}", m.input.kind);
        if !multiple
            && !out.is_empty()
            && seen_kinds.contains(&kind)
            && matched.iter().all(|v| covered_terms.contains(v))
        {
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
        covered_terms.extend(matched);
        seen_kinds.insert(kind);
        seen_content.insert(fold(&m.input.content));
        if let Some(key) = key {
            seen_property.insert(key);
        }
        out.push(m);
        if out.len() >= if multiple { 12 } else { 3 } {
            break;
        }
    }
    Ok((out, used))
}
