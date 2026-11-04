//! Read-only contextual projection of canonical quality, review and provenance.
//! Modality and validity are separate: a past plan never becomes past execution.
use crate::{
    db_error, intelligent::quality, Kind, Memory, Reference, Result, Source, Store, TurnRequest,
};
use rusqlite::{params, Transaction};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FactEvidence {
    pub role: String,
    pub text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContextFact {
    pub id: String,
    pub version: i64,
    pub entity: String,
    pub property: String,
    pub value: String,
    pub modality: String,
    pub validity: String,
    pub scope: String,
    pub project: String,
    pub review: String,
    pub facets: Vec<String>,
    pub evidence: Vec<FactEvidence>,
}
/// A project state qualifies its technical fields. Keep this small authoritative
/// qualifier ahead of fields so budget trimming cannot expose execution alone.
pub(crate) fn append_project_state(
    tx: &Transaction,
    req: &TurnRequest,
    memories: &mut Vec<Memory>,
) -> Result<()> {
    let entity = format!("project.{}", req.project);
    let mut selected = false;
    for m in memories.iter() {
        selected |= quality(tx, &m.input.id)?.is_some_and(|q| q.subject == entity);
    }
    let query = crate::extraction::fold(&req.query);
    let question = ["linguagem", "language", "database", "banco"]
        .iter()
        .any(|s| query.contains(s))
        || (["projeto", "project"].iter().any(|s| query.contains(s))
            && [
                "status",
                "estado",
                "andamento",
                "cancel",
                "conclu",
                "complete",
                "desenvolv",
                "implement",
                "execut",
                "pronto",
                "progress",
                "build",
                "using",
                " usa",
            ]
            .iter()
            .any(|s| query.contains(s)));
    if !selected && !question {
        return Ok(());
    }
    let mut stmt = tx.prepare(&format!("SELECT {} FROM memories WHERE scope=?1 AND project=?2 AND state='active' AND id IN (SELECT memory_id FROM memory_quality WHERE subject=?3 AND property='lifecycle') ORDER BY updated_at DESC,id LIMIT 1", crate::COLS)).map_err(db_error)?;
    let records = stmt
        .query_map(params![req.scope, req.project, entity], crate::row)
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;
    for m in records {
        if !memories.iter().any(|old| old.input.id == m.input.id)
            && crate::retrieval::eligible(tx, &m, &req.query, crate::now())?
        {
            memories.insert(0, m);
        }
    }
    Ok(())
}
pub(crate) fn project(tx: &Transaction, memories: &[Memory]) -> Result<Vec<ContextFact>> {
    let mut facts = Vec::new();
    for m in memories {
        let Some(q) = quality(tx, &m.input.id)? else {
            continue;
        };
        let reviewed: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM history WHERE memory_id=?1 AND operation='approve')",
                [&m.input.id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        let modality = match q.temporal.as_str() {
            "planned" => "planned",
            "completed" => "completed",
            "cancelled" => "cancelled",
            "in_progress" => "in_progress",
            "unknown" => "explicitly_unknown",
            "hypothesis" => "hypothesis",
            "temporary" => "temporary_observation",
            "preference" => "preference",
            _ if q.property == "description" && q.subject.starts_with("project.") => "project_description",
            _ if m.input.kind == Kind::Decision => "decision",
            _ if m.input.kind == Kind::Goal => "planned",
            _ if m.input.kind == Kind::Task => "task",
            _ => "current_fact",
        }
        .to_string();
        let mut evidence = Vec::new();
        let mut stmt = tx.prepare("SELECT evidence FROM memory_provenance WHERE memory_id=?1 ORDER BY at DESC,interaction_id LIMIT 2").map_err(db_error)?;
        for item in stmt
            .query_map([&m.input.id], |r| r.get::<_, String>(0))
            .map_err(db_error)?
        {
            evidence.push(FactEvidence {
                role: "current_source".into(),
                text: item.map_err(db_error)?.chars().take(300).collect(),
            });
        }
        if let Some(old_id) = &q.replaces_id {
            if evidence.len() < 3 {
                let previous: Option<String> = tx.query_row(
                    "SELECT p.evidence FROM memory_provenance p JOIN memory_quality q ON q.memory_id=p.memory_id WHERE p.memory_id=?1 AND q.subject=?2 AND q.property=?3 ORDER BY p.at DESC LIMIT 1",
                    params![old_id,q.subject,q.property], |r| r.get(0),
                ).optional().map_err(db_error)?;
                if let Some(text) = previous {
                    evidence.push(FactEvidence {
                        role: "previous_property_link_not_current_value".into(),
                        text: text.chars().take(300).collect(),
                    });
                }
            }
        }
        let ended_after_field_recorded = if crate::dialogue::execution_track(
            &q.subject,
            &q.property,
            &q.temporal,
        ) {
            tx.query_row("SELECT EXISTS(SELECT 1 FROM memories n JOIN memory_quality state ON state.memory_id=n.id WHERE n.scope=?1 AND n.project=?2 AND n.state='active' AND state.subject=?3 AND state.property='lifecycle' AND state.temporal IN ('cancelled','completed') AND n.updated_at>=?4)", params![m.input.scope,m.input.project,q.subject,m.updated_at], |r|r.get::<_,bool>(0)).map_err(db_error)?
        } else {
            false
        };
        facts.push(ContextFact {
            id: m.input.id.clone(),
            version: m.version,
            entity: q.subject,
            property: q.property,
            value: q.value,
            modality,
            validity: if m.state == crate::Status::Obsolete || ended_after_field_recorded {
                "historical"
            } else {
                "current"
            }
            .into(),
            scope: m.input.scope.clone(),
            project: m.input.project.clone(),
            review: if reviewed {
                "reviewed"
            } else if m.input.source == Source::ReviewedImport {
                "reviewed_import"
            } else {
                "independently_grounded_active"
            }
            .into(),
            facets: q.facets,
            evidence,
        });
    }
    Ok(facts)
}

/// Ephemeral evidence from this USER turn, never a second persistent memory store.
/// Acknowledging what the user just said does not approve it as canonical memory.
pub(crate) fn statements(req: &TurnRequest) -> Vec<ContextFact> {
    crate::dialogue::extract(&req.query, &req.project)
        .into_iter()
        .enumerate()
        .map(|(i, c)| {
            let modality = match c.temporal.as_str() {
                "planned" => "planned",
                "completed" => "completed",
                "cancelled" => "cancelled",
                "in_progress" => "in_progress",
                "unknown" => "explicitly_unknown",
                "hypothesis" => "hypothesis",
                "temporary" => "temporary_observation",
                "preference" => "preference",
                _ if c.property == "description" && c.subject.starts_with("project.") => "project_description",
                _ if c.kind == Kind::Decision => "decision",
                _ if c.kind == Kind::Goal => "planned",
                _ if c.kind == Kind::Task => "task",
                _ => "current_fact",
            }
            .into();
            ContextFact {
                id: format!("current-turn-{i}"),
                version: 0,
                entity: c.subject,
                property: c.property,
                value: c.value,
                modality,
                validity: "current_user_statement".into(),
                scope: req.scope.clone(),
                project: req.project.clone(),
                review: "current_turn_not_reviewed_or_persisted".into(),
                facets: c.facets,
                evidence: vec![FactEvidence {
                    role: "current_user_statement".into(),
                    text: c.evidence,
                }],
            }
        })
        .collect()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnswerEvidence {
    pub facts: Vec<ContextFact>,
    pub current_statements: Vec<ContextFact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directive_receipt: Option<serde_json::Value>,
    pub has_scoped_records: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ClaimedField {
    pub entity: String,
    pub property: String,
    pub value: String,
}
impl Store {
    /// Recheck authority immediately before exposing generated text. A deletion,
    /// correction or unresolved conflict during generation invalidates the old snapshot.
    pub fn answer_evidence(
        &mut self,
        req: &TurnRequest,
        references: &[Reference],
    ) -> Result<AnswerEvidence> {
        self.answer_evidence_for_claims(req, references, &[])
    }
    /// Optional bounded confirmation of a generated assertion against canonical
    /// state. This is verification evidence, never an initial retrieval hit.
    pub fn answer_evidence_for_claims(
        &mut self,
        req: &TurnRequest,
        references: &[Reference],
        claims: &[ClaimedField],
    ) -> Result<AnswerEvidence> {
        self.answer_evidence_for_source(req, references, claims, None)
    }
    pub fn answer_evidence_for_source(
        &mut self,
        req: &TurnRequest,
        references: &[Reference],
        claims: &[ClaimedField],
        source: Option<&crate::turn_sources::SourceProof>,
    ) -> Result<AnswerEvidence> {
        crate::identifier(&req.scope)?;
        crate::identifier(&req.project)?;
        if references.len() > 12 || claims.len() > 16 || req.query.len() > 32000 {
            return Err("Answer evidence ceiling".into());
        }
        let tx = self.db.transaction().map_err(db_error)?;
        let mut memories = Vec::new();
        for reference in references {
            let record=tx.query_row(&format!("SELECT {} FROM memories WHERE id=?1 AND scope=?2 AND project=?3 AND (state='active' OR (state='obsolete' AND ?5)) AND version=?4",crate::COLS),params![reference.id,req.scope,req.project,reference.version,crate::retrieval::historical_intent(&req.query)],crate::row).optional().map_err(db_error)?;
            if let Some(m) = record {
                if crate::retrieval::eligible(&tx, &m, &req.query, crate::now())? {
                    memories.push(m);
                }
            }
        }
        for claim in claims {
            if ![
                "user",
                "device.primary_computer",
                "device.work_laptop",
                "device.other_computer",
            ]
            .contains(&claim.entity.as_str())
                && claim.entity != format!("project.{}", req.project)
            {
                return Err("Claimed field entity is outside this scope".into());
            }
            if claim.value.len() > 128
                || ![
                    "language",
                    "database",
                    "ram_capacity",
                    "free_ram",
                    "coffee_sugar",
                    "interface_theme",
                    "communication_style",
                    "response_language",
                    "favorite_beverage",
                    "communication_focus",
                    "games_tactics",
                    "games_turn_strategy",
                    "games_character_creation",
                    "games_class_selection",
                    "interface_clutter",
                    "music_instrument",
                    "hobby",
                    "book_genre",
                    "outing",
                    "photo_paper",
                    "shelf_material",
                ]
                .contains(&claim.property.as_str())
            {
                return Err("Unsupported claimed field".into());
            }
            let mut stmt=tx.prepare(&format!("SELECT {} FROM memories WHERE id IN (SELECT memory_id FROM memory_quality WHERE subject=?3 AND property=?4 AND value=?5) AND scope=?1 AND project=?2 AND state IN ('active','obsolete') ORDER BY updated_at DESC,id LIMIT 2",crate::COLS)).map_err(db_error)?;
            let records = stmt
                .query_map(
                    params![
                        req.scope,
                        req.project,
                        claim.entity,
                        claim.property,
                        claim.value
                    ],
                    crate::row,
                )
                .map_err(db_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(db_error)?;
            for m in records {
                if !memories.iter().any(|old| old.input.id == m.input.id)
                    // An exact generated field is a separate canonical lookup,
                    // not another initial query-relevance injection. Preserve
                    // current-source conflicts, expiry, review and deletion gates.
                    && crate::retrieval::eligible_claim(&tx, &m,
                        &if m.state==crate::Status::Obsolete { format!("{} historical",req.query) } else { req.query.clone() }, crate::now())?
                {
                    memories.push(m)
                }
            }
        }
        append_project_state(&tx, req, &mut memories)?;
        let facts = project(&tx, &memories)?;
        let current_statements = if let Some(source) = source {
            crate::turn_sources::statements(&tx, req, source)?
        } else {
            statements(req)
        };
        let directive_receipt = if let Some(source) = source {
            crate::turn_sources::receipt(&tx, &source.interaction)?
        } else {
            None
        };
        let has_scoped_records = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM memories WHERE scope=?1 AND project=?2)",
                rusqlite::params![req.scope, req.project],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(AnswerEvidence {
            facts,
            current_statements,
            directive_receipt,
            has_scoped_records,
        })
    }
}
use rusqlite::OptionalExtension;
