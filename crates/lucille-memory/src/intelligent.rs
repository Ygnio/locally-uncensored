//! Persistent bounded learning. No inference, networking or assistant/tool ingestion.
use crate::extraction::{digest, fold, preference_content};
pub use crate::extraction::{extract, Claim};
use crate::{
    db_error, identifier, now, row, types, Action, ConfigurationPermit, Kind, Memory, Result,
    Store, COLS,
};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use sha2::Digest;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LearningPolicy {
    pub enabled: bool,
    pub autonomous_preferences: bool,
    pub automatic_indexing: bool,
    pub extraction_mode: crate::llm::ExtractionMode,
    pub auto_kinds: Vec<Kind>,
    pub minimum_confidence: f64,
    pub minimum_relevance: f64,
    pub max_pending: usize,
    pub lexical_weight: f64,
    pub semantic_weight: f64,
    pub minimum_similarity: f64,
}
impl Default for LearningPolicy {
    fn default() -> Self {
        Self {
            enabled: false,
            autonomous_preferences: false,
            automatic_indexing: false,
            extraction_mode: Default::default(),
            auto_kinds: vec![Kind::Preference],
            minimum_confidence: 0.96,
            minimum_relevance: 0.8,
            max_pending: 64,
            lexical_weight: 0.7,
            semantic_weight: 0.3,
            minimum_similarity: 0.78,
        }
    }
}
impl LearningPolicy {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.max_pending == 0
            || self.max_pending > 64
            || !self.minimum_confidence.is_finite()
            || !(0.96..=1.0).contains(&self.minimum_confidence)
            || !self.minimum_relevance.is_finite()
            || !(0.8..=1.0).contains(&self.minimum_relevance)
            || self.auto_kinds.iter().any(|k| *k != Kind::Preference)
            || ![
                self.lexical_weight,
                self.semantic_weight,
                self.minimum_similarity,
            ]
            .iter()
            .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
            || (self.lexical_weight + self.semantic_weight - 1.0).abs() > 0.001
        {
            return Err("Invalid conservative learning policy".into());
        }
        Ok(())
    }
    fn promotes(&self, c: &Claim) -> bool {
        if !crate::source_authority::valid(c) {return false}
        if self.autonomous_preferences {
            return self.enabled
                && self.auto_kinds.contains(&Kind::Preference)
                && !c.rule.contains("renewal")
                && crate::dialogue::safe_preference(c);
        }
        self.enabled
            && self.auto_kinds.contains(&c.kind)
            && c.confidence >= self.minimum_confidence
            && c.relevance >= self.minimum_relevance
            && c.temporal == "preference"
            && c.rule.starts_with("explicit_")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interaction {
    pub conversation_id: String,
    pub message_id: String,
    pub scope: String,
    pub project: String,
    pub mode: crate::Mode,
    pub role: String,
    pub user_text: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub state: String,
    pub attempts: i64,
    pub created_at: i64,
    pub updated_at: i64,
    pub error_code: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Lease {
    pub id: String,
    pub token: String,
    pub scope: String,
    pub project: String,
    pub claims: Vec<Claim>,
    pub attempts: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Quality {
    pub subject: String,
    pub property: String,
    pub value: String,
    pub confidence: f64,
    pub relevance: f64,
    pub temporal: String,
    pub expires_at: Option<i64>,
    pub priority: i64,
    pub replaces_id: Option<String>,
    pub replaces_version: Option<i64>,
    pub facets: Vec<String>,
    pub evidence_rule: String,
}
impl Quality {
    fn from_claim(c: &Claim, at: i64) -> Self {
        Self {
            subject: c.subject.clone(),
            property: c.property.clone(),
            value: c.value.clone(),
            confidence: c.confidence,
            relevance: c.relevance,
            temporal: c.temporal.clone(),
            expires_at: c.ttl_seconds.map(|ttl| at + ttl),
            priority: if c.correction { 2 } else { 1 },
            replaces_id: None,
            replaces_version: None,
            facets: c.facets.clone(),
            evidence_rule: c.authority.as_ref().map(|proof|
                format!("{}|native_source_authority_v2:{}", c.rule, serde_json::to_string(proof).expect("bounded source proof")))
                .unwrap_or_else(|| c.rule.clone()),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Detail {
    pub memory: Memory,
    pub quality: Option<Quality>,
    pub provenance: Vec<Provenance>,
    pub revisions: Vec<i64>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provenance {
    pub interaction_id: String,
    pub evidence: String,
    pub at: i64,
}
#[derive(Serialize, Deserialize)]
struct Snapshot {
    memory: Memory,
    quality: Option<Quality>,
}

pub(crate) fn quality(db: &Connection, id: &str) -> Result<Option<Quality>> {
    db.query_row("SELECT subject,property,value,confidence,relevance,temporal,expires_at,priority,replaces_id,replaces_version,facets_json,evidence_rule FROM memory_quality WHERE memory_id=?1",[id],|r|{let facets:String=r.get(10)?;Ok(Quality{subject:r.get(0)?,property:r.get(1)?,value:r.get(2)?,confidence:r.get(3)?,relevance:r.get(4)?,temporal:r.get(5)?,expires_at:r.get(6)?,priority:r.get(7)?,replaces_id:r.get(8)?,replaces_version:r.get(9)?,facets:serde_json::from_str(&facets).map_err(|e|rusqlite::Error::FromSqlConversionFailure(10,rusqlite::types::Type::Text,Box::new(e)))?,evidence_rule:r.get(11)?})}).optional().map_err(db_error)
}
fn put_quality(tx: &Transaction, id: &str, q: &Quality) -> Result<()> {
    tx.execute(
        "INSERT OR REPLACE INTO memory_quality VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
        params![
            id,
            q.subject,
            q.property,
            q.value,
            q.confidence,
            q.relevance,
            q.temporal,
            q.expires_at,
            q.priority,
            q.replaces_id,
            q.replaces_version,
            serde_json::to_string(&q.facets).map_err(|_| "Invalid facets")?,
            q.evidence_rule
        ],
    )
    .map_err(db_error)?;
    Ok(())
}
pub(crate) fn snapshot(tx: &Transaction, m: &Memory) -> Result<()> {
    tx.execute(
        "DELETE FROM memory_vectors WHERE memory_id=?1",
        [&m.input.id],
    )
    .map_err(db_error)?;
    let s = Snapshot {
        memory: m.clone(),
        quality: quality(tx, &m.input.id)?,
    };
    tx.execute(
        "INSERT OR IGNORE INTO memory_revisions VALUES(?1,?2,?3,?4)",
        params![
            m.input.id,
            m.version,
            serde_json::to_string(&s).map_err(|_| "Invalid revision")?,
            now()
        ],
    )
    .map_err(db_error)?;
    Ok(())
}
pub(crate) fn before_change(tx: &Transaction, m: &Memory, action: &Action) -> Result<()> {
    if matches!(action, Action::Approve)
        && quality(tx, &m.input.id)?.is_some_and(|q| {
            q.subject == "device.unspecified"
                || ["capacity_update", "equipment_change"].contains(&q.property.as_str())
        })
    {
        return Err("Clarify equipment and capacity property before approval".into());
    }
    if matches!(action, Action::Delete) {
        let fingerprint = quality(tx, &m.input.id)?
            .map(|q| digest(&format!("{}|{}|{}", q.subject, q.property, q.value)));
        tx.execute(
            "INSERT OR IGNORE INTO memory_tombstones VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                m.input.id,
                m.input.scope,
                m.input.project,
                types::label(&m.input.kind),
                fingerprint,
                now()
            ],
        )
        .map_err(db_error)?;
    } else {
        snapshot(tx, m)?
    }
    if matches!(action, Action::Approve) {
        if let Some(q) = quality(tx, &m.input.id)? {
            let contextual_correction = q.evidence_rule.contains(crate::evidence::CONTEXTUAL_RAM_CORRECTION_RULE);
            let replacement = q.replaces_id.zip(q.replaces_version);
            if contextual_correction && replacement.is_none() {
                return Err("Replacement source changed; review again".into());
            }
            if let Some((id, version)) = replacement {
                let prior = tx
                    .query_row(
                        &format!(
                            "SELECT {COLS} FROM memories WHERE id=?1 AND scope=?2 AND project=?3"
                        ),
                        params![id, m.input.scope, m.input.project],
                        row,
                    )
                    .optional()
                    .map_err(db_error)?;
                if contextual_correction && !prior.as_ref().is_some_and(|p| p.state == crate::Status::Active && p.version == version) {
                    return Err("Replacement source changed; review again".into());
                }
                if let Some(prior) = prior {
                    if prior.state != crate::Status::Obsolete {
                        if prior.version != version {
                            return Err("Replacement source changed; review again".into());
                        }
                        snapshot(tx, &prior)?;
                        tx.execute("UPDATE memories SET state='obsolete',version=version+1,updated_at=?2 WHERE id=?1",params![id,now()]).map_err(db_error)?;
                        tx.execute("INSERT INTO history(memory_id,record_id,operation,version,at) VALUES(?1,?1,'obsolete',?2,?3)",params![id,version+1,now()]).map_err(db_error)?;
                    }
                }
            }
        }
        if let Some(q) = quality(tx, &m.input.id)? {
            let unresolved:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memory_quality q JOIN memories n ON n.id=q.memory_id WHERE n.id!=?1 AND n.scope=?2 AND n.project=?3 AND n.state='active' AND q.subject=?4 AND q.property=?5 AND q.value!=?6 AND (?7=0 OR (q.temporal IN ('in_progress','current','completed','cancelled'))=?8))",params![m.input.id,m.input.scope,m.input.project,q.subject,q.property,q.value,q.subject.starts_with("project.") && ["language","database"].contains(&q.property.as_str()),crate::dialogue::execution_track(&q.subject,&q.property,&q.temporal)],|r|r.get(0)).map_err(db_error)?;
            if unresolved {
                return Err("Review other current claims for this subject first".into());
            }
        }
    }
    Ok(())
}
pub(crate) fn after_update(tx: &Transaction, m: &Memory, content: &str) -> Result<()> {
    let old = quality(tx, &m.input.id)?;
    if let Some(c) = extract(content).into_iter().next() {
        let mut next = Quality::from_claim(&c, now());
        if let Some(old) = old.filter(|old| old.subject == c.subject && old.property == c.property)
        {
            next.replaces_id = old.replaces_id;
            next.replaces_version = old.replaces_version;
        }
        put_quality(tx, &m.input.id, &next)?
    } else if let Some(mut q) = old {
        q.value = digest(content);
        q.evidence_rule = "manual_revision".into();
        put_quality(tx, &m.input.id, &q)?
    }
    Ok(())
}
impl Store {
    pub fn learning_policy(&self) -> Result<LearningPolicy> {
        let raw: Option<String> = self
            .db
            .query_row("SELECT json FROM learning_policy WHERE id=1", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(db_error)?;
        raw.map(|s| serde_json::from_str(&s).map_err(|_| "Invalid stored policy".into()))
            .unwrap_or_else(|| Ok(LearningPolicy::default()))
    }
    pub fn set_learning_policy(
        &mut self,
        policy: LearningPolicy,
        _permit: &ConfigurationPermit,
    ) -> Result<()> {
        policy.validate()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let old: Option<String> = tx
            .query_row("SELECT json FROM learning_policy WHERE id=1", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(db_error)?;
        let old: LearningPolicy = old
            .map(|s| serde_json::from_str(&s).map_err(|_| "Invalid stored policy"))
            .transpose()?
            .unwrap_or_default();
        let changed_mode = old.extraction_mode != policy.extraction_mode;
        tx.execute(
            "INSERT OR REPLACE INTO learning_policy VALUES(1,?1)",
            [serde_json::to_string(&policy).map_err(|_| "Invalid policy")?],
        )
        .map_err(db_error)?;
        if !policy.enabled || changed_mode {
            tx.execute("UPDATE extraction_jobs SET state='cancelled',evidence_json='[]',lease_token=NULL,lease_until=NULL,updated_at=?1 WHERE state IN ('queued','retry','running')",[now()]).map_err(db_error)?;
        }
        tx.commit().map_err(db_error)
    }
    pub fn enqueue_learning(&mut self, input: Interaction) -> Result<serde_json::Value> {
        self.enqueue_learning_phase(input, false)
    }
    pub fn prepare_user_turn(&mut self, input: Interaction) -> Result<serde_json::Value> {
        self.identity()?;
        self.enqueue_learning_phase(input, true)
    }
    fn enqueue_learning_phase(
        &mut self,
        input: Interaction,
        deferred: bool,
    ) -> Result<serde_json::Value> {
        for id in [
            &input.conversation_id,
            &input.message_id,
            &input.scope,
            &input.project,
        ] {
            identifier(id)?
        }
        if input.role != "user" {
            return Ok(serde_json::json!({"state":"ignored","reason":"user_only"}));
        }
        let policy = self.learning_policy()?;
        if !policy.enabled {
            return Ok(serde_json::json!({"state":"disabled"}));
        }
        if policy.autonomous_preferences {
            if let Some(result) = self.forget_user_directive(&input)? {
                return Ok(result);
            }
        }
        let mut claims = if policy.autonomous_preferences {
            crate::dialogue::extract(&input.user_text, &input.project)
        } else {
            extract(&input.user_text)
        };
        claims = claims.into_iter().filter_map(|c|crate::source_authority::attest(&input.user_text,c)).collect();
        claims.retain(|c| {
            c.project_hint
                .as_ref()
                .is_none_or(|p| fold(p) == fold(&input.project))
        });
        let mut seen = std::collections::HashSet::new();
        claims.retain(|c| seen.insert((c.fingerprint(), c.temporal.clone(), c.facets.clone())));
        let use_llm = policy.extraction_mode != crate::llm::ExtractionMode::Rules
            && if policy.autonomous_preferences {
                crate::dialogue::eligible_source(&input.user_text)
            } else {
                crate::llm::eligible_source(&input.user_text)
            };
        if (claims.is_empty() && !use_llm)
            || (policy.extraction_mode == crate::llm::ExtractionMode::Llm && !use_llm)
        {
            return Ok(serde_json::json!({"state":"ignored","reason":"not_eligible"}));
        }
        let id = digest(
            &serde_json::to_string(&(
                &input.conversation_id,
                &input.message_id,
                &input.scope,
                &input.project,
                digest(&input.user_text),
            ))
            .map_err(|_| "Invalid interaction")?,
        );
        if let Some(job) = self.learning_job(&id)? {
            if job.state == "queued" {
                let held: bool = self
                    .db
                    .query_row(
                        "SELECT lease_token IS NOT NULL FROM extraction_jobs WHERE id=?1",
                        [&id],
                        |r| r.get(0),
                    )
                    .map_err(db_error)?;
                if held && !deferred {
                    self.recover_pending_user_turns(now())?;
                    self.db.execute("UPDATE extraction_jobs SET next_at=?2,lease_token=NULL,lease_until=NULL,updated_at=?2 WHERE id=?1 AND state='queued' AND lease_token=?3 AND lease_until>?2",params![id,now(),crate::turn_sources::owner()]).map_err(db_error)?;
                    return Ok(serde_json::json!(self.learning_job(&id)?.unwrap()));
                }
            }
            return Ok(serde_json::json!(job));
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let pending: i64 = tx
            .query_row(
                "SELECT count(*) FROM extraction_jobs WHERE state IN ('queued','running','retry')",
                [],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if pending >= policy.max_pending as i64 {
            return Err("Learning queue is full".into());
        }
        tx.execute("INSERT OR IGNORE INTO extraction_jobs(id,conversation_id,message_id,scope,project,mode,state,evidence_json,created_at,updated_at,next_at) VALUES(?1,?2,?3,?4,?5,?6,'queued',?7,?8,?8,?8)",params![id,input.conversation_id,input.message_id,input.scope,input.project,types::label(&input.mode),serde_json::to_string(&claims).map_err(|_|"Invalid evidence")?,now()]).map_err(db_error)?;
        crate::source_operations::reserve(&tx, &id, None)?;
        if deferred {
            tx.execute(
                "UPDATE extraction_jobs SET next_at=?2,lease_token=?3,lease_until=?4 WHERE id=?1",
                params![id, now() + 3600, crate::turn_sources::owner(), now() + 900],
            )
            .map_err(db_error)?;
        }
        if use_llm {
            tx.execute(
                "INSERT INTO learning_inputs VALUES(?1,?2,?3)",
                params![
                    id,
                    input.user_text,
                    if policy.extraction_mode == crate::llm::ExtractionMode::Llm {
                        "llm"
                    } else {
                        "hybrid"
                    }
                ],
            )
            .map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        Ok(serde_json::json!(self.learning_job(&id)?.unwrap()))
    }
    pub fn learning_job(&self, id: &str) -> Result<Option<Job>> {
        self.db.query_row("SELECT id,state,attempts,created_at,updated_at,error_code FROM extraction_jobs WHERE id=?1",[id],job_row).optional().map_err(db_error)
    }
    pub fn learning_jobs(&self, scope: &str, project: &str) -> Result<Vec<Job>> {
        identifier(scope)?;
        identifier(project)?;
        let mut q=self.db.prepare("SELECT id,state,attempts,created_at,updated_at,error_code FROM extraction_jobs WHERE scope=?1 AND project=?2 ORDER BY created_at DESC,id DESC LIMIT 50").map_err(db_error)?;
        let rows = q
            .query_map(params![scope, project], job_row)
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error);
        rows
    }
    pub fn claim_learning(&mut self, at: i64) -> Result<Option<Lease>> {
        self.claim_learning_kind(at, false)
    }
    pub fn claim_llm_learning(&mut self, at: i64) -> Result<Option<Lease>> {
        self.claim_learning_kind(at, true)
    }
    fn claim_learning_kind(&mut self, at: i64, llm: bool) -> Result<Option<Lease>> {
        self.recover_pending_user_turns(at)?;
        if !self.learning_policy()?.enabled {
            return Ok(None);
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        tx.execute("UPDATE extraction_jobs SET state='failed',evidence_json='[]',error_code='attempt_limit',lease_token=NULL WHERE state='running' AND lease_until<=?1 AND attempts>=3",[at]).map_err(db_error)?;
        let data:Option<(String,String,String,String,i64)>=tx.query_row("SELECT id,scope,project,evidence_json,attempts FROM extraction_jobs WHERE attempts<3 AND (EXISTS(SELECT 1 FROM learning_inputs i WHERE i.job_id=extraction_jobs.id))=?2 AND ((state IN ('queued','retry') AND next_at<=?1) OR (state='running' AND lease_until<=?1)) ORDER BY next_at,id LIMIT 1",params![at,llm],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(db_error)?;
        let Some((id, scope, project, raw, attempts)) = data else {
            tx.commit().map_err(db_error)?;
            return Ok(None);
        };
        let token = uuid::Uuid::new_v4().to_string();
        let claims = serde_json::from_str(&raw).map_err(|_| "Invalid queue evidence")?;
        tx.execute("UPDATE extraction_jobs SET state='running',attempts=attempts+1,lease_token=?2,lease_until=?3,updated_at=?4 WHERE id=?1",params![id,token,at+30,at]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(Some(Lease {
            id,
            token,
            scope,
            project,
            claims,
            attempts: attempts + 1,
        }))
    }
    pub fn learning_input(&self, lease: &Lease) -> Result<Option<(String, String)>> {
        self.db.query_row("SELECT i.user_text,i.extraction_mode FROM learning_inputs i JOIN extraction_jobs j ON j.id=i.job_id WHERE j.id=?1 AND j.lease_token=?2 AND j.state='running'",params![lease.id,lease.token],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(db_error)
    }
    pub fn complete_llm_learning(&mut self, lease: &Lease, json: &str, at: i64) -> Result<bool> {
        let owned: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM extraction_jobs WHERE id=?1 AND state='running' AND lease_token=?2 AND scope=?3 AND project=?4 AND lease_until>?5)",params![lease.id,lease.token,lease.scope,lease.project,at],|r|r.get(0)).map_err(db_error)?;
        let result = self.complete_learning_output(lease, &[], at, Some(json));
        if owned
            && result
                .as_ref()
                .err()
                .is_some_and(|error| error != "Lucille database operation failed")
        {
            // The failed transaction cannot retain write authority until lease expiry.
            // Bounded retry/failure also purges source input on the terminal attempt.
            let source_rejected = result
                .as_ref()
                .err()
                .is_some_and(|e| e == "Ineligible V3 source")
                && self.learning_input(lease)?.is_some_and(|(text, _)| {
                    !crate::dialogue::eligible_source(&text)
                        || !crate::dialogue::project_scope_matches(&text, &lease.project)
                });
            self.fail_learning_classified(lease, at, source_rejected)?;
        }
        result
    }
    pub fn complete_learning(
        &mut self,
        lease: &Lease,
        proposals: &[Claim],
        at: i64,
    ) -> Result<bool> {
        self.complete_learning_output(lease, proposals, at, None)
    }
    fn complete_learning_output(
        &mut self,
        lease: &Lease,
        proposals: &[Claim],
        at: i64,
        llm: Option<&str>,
    ) -> Result<bool> {
        if llm.is_none()
            && (proposals.len() > 4
                || proposals.iter().any(|c| {
                    !lease.claims.contains(c)
                        || !crate::source_authority::valid(c)
                        || !crate::source_authority::independently_interpreted(c,&lease.project)
                }))
        {
            return Err("Unverified extraction proposal".into());
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let raw: Option<String> = tx
            .query_row("SELECT json FROM learning_policy WHERE id=1", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(db_error)?;
        let policy: LearningPolicy = raw
            .map(|v| serde_json::from_str(&v).map_err(|_| "Invalid policy"))
            .transpose()?
            .unwrap_or_default();
        policy.validate()?;
        let stored:Option<(String,String,String)>=tx.query_row("SELECT scope,project,evidence_json FROM extraction_jobs WHERE id=?1 AND state='running' AND lease_token=?2 AND lease_until>?3",params![lease.id,lease.token,at],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(db_error)?;
        let Some((scope, project, evidence)) = stored else {
            return Ok(false);
        };
        if !policy.enabled {
            return Ok(false);
        }
        let verified: Vec<Claim> =
            serde_json::from_str(&evidence).map_err(|_| "Invalid queue evidence")?;
        if scope != lease.scope
            || project != lease.project
            || proposals.iter().any(|c| !verified.contains(c))
        {
            return Err("Queue evidence or scope mismatch".into());
        }
        let input: Option<(String, String)> = tx
            .query_row(
                "SELECT user_text,extraction_mode FROM learning_inputs WHERE job_id=?1",
                [&lease.id],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()
            .map_err(db_error)?;
        let mut grounded = vec![];
        if let Some(json) = llm {
            let (text, mode) = input.as_ref().ok_or("Missing LLM source")?;
            grounded = crate::llm::validate(&text, &scope, &project, json)?
                .into_iter().filter_map(|c|crate::source_authority::attest(text,c)).collect();
            if mode == "hybrid" {
                for c in verified.iter().cloned().filter(|c|crate::source_authority::independently_interpreted(c,&project)).filter_map(|c|crate::source_authority::attest(text,c)) {
                    if !grounded.iter().any(|v| {
                        v.fingerprint() == c.fingerprint()
                            && v.temporal == c.temporal
                            && v.facets == c.facets
                    }) {
                        grounded.push(c)
                    }
                }
            }
            if grounded.len() > 4 {
                return Err("Hybrid proposal ceiling exceeded".into());
            }
        } else if input.is_some() {
            return Err("LLM jobs require structured validation".into());
        }
        let proposals = if llm.is_some() {
            &grounded[..]
        } else {
            proposals
        };
        let mut ordered = proposals.to_vec();
        ordered.sort_by_key(|claim| {
            input
                .as_ref()
                .and_then(|(text, _)| text.find(&claim.evidence))
                .or_else(|| verified.iter().position(|c| c.evidence == claim.evidence))
                .unwrap_or(usize::MAX)
        });
        for (index, claim) in ordered.iter().enumerate() {
            let ambiguous = ordered.iter().enumerate().any(|(other, c)| {
                other != index
                    && c.subject == claim.subject
                    && c.property == claim.property
                    && c.value != claim.value
                    && !crate::dialogue::separate_tracks(
                        &claim.subject,
                        &claim.property,
                        &claim.temporal,
                        &c.temporal,
                    )
            }) && !ordered.iter().enumerate().any(|(later, c)| {
                later > 0
                    && c.subject == claim.subject
                    && c.property == claim.property
                    && c.correction
            });
            if ambiguous {
                let mut review_policy = policy.clone();
                review_policy.autonomous_preferences = false;
                review_policy.auto_kinds.clear();
                ingest(&tx, lease, claim, &review_policy, at)?;
            } else {
                ingest(&tx, lease, claim, &policy, at)?
            }
        }
        tx.execute("UPDATE extraction_jobs SET state='done',evidence_json='[]',lease_token=NULL,lease_until=NULL,error_code=NULL,updated_at=?2 WHERE id=?1",params![lease.id,at]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(true)
    }
    pub fn fail_learning(&mut self, lease: &Lease, at: i64) -> Result<()> {
        self.fail_learning_classified(lease, at, false)
    }
    fn fail_learning_classified(
        &mut self,
        lease: &Lease,
        at: i64,
        source_rejected: bool,
    ) -> Result<()> {
        let attempts:Option<i64>=self.db.query_row("SELECT attempts FROM extraction_jobs WHERE id=?1 AND state='running' AND lease_token=?2 AND lease_until>?3",params![lease.id,lease.token,at],|r|r.get(0)).optional().map_err(db_error)?;
        let Some(attempts) = attempts else {
            return Ok(());
        };
        // Repeating a deterministic rejection of the original source cannot
        // establish authority. Transport/model-output failures keep the old cap.
        let terminal = source_rejected || attempts >= 3;
        self.db.execute("UPDATE extraction_jobs SET state=?3,error_code=?7,next_at=?4,updated_at=?5,lease_token=NULL,lease_until=NULL,evidence_json=CASE WHEN ?6 THEN '[]' ELSE evidence_json END WHERE id=?1 AND state='running' AND lease_token=?2",params![lease.id,lease.token,if terminal{"failed"}else{"retry"},at+(1<<attempts),at,terminal,if source_rejected{"source_ineligible"}else{"extractor_failed"}]).map_err(db_error)?;
        Ok(())
    }
    pub fn cancel_learning(&mut self, id: &str, _permit: &ConfigurationPermit) -> Result<bool> {
        identifier(id)?;
        let n=self.db.execute("UPDATE extraction_jobs SET state='cancelled',evidence_json='[]',lease_token=NULL,lease_until=NULL,updated_at=?2 WHERE id=?1 AND state IN ('queued','running','retry')",params![id,now()]).map_err(db_error)?;
        Ok(n == 1)
    }
    pub fn memory_details(&self, scope: &str, project: &str) -> Result<Vec<Detail>> {
        self.list(scope,project)?.into_iter().map(|memory|{
            let mut q=self.db.prepare("SELECT interaction_id,evidence,at FROM memory_provenance WHERE memory_id=?1 ORDER BY at,interaction_id").map_err(db_error)?;
            let provenance=q.query_map([&memory.input.id],|r|Ok(Provenance{interaction_id:r.get(0)?,evidence:r.get(1)?,at:r.get(2)?})).map_err(db_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(db_error)?;
            let mut q=self.db.prepare("SELECT version FROM memory_revisions WHERE memory_id=?1 ORDER BY version DESC").map_err(db_error)?;
            let revisions=q.query_map([&memory.input.id],|r|r.get(0)).map_err(db_error)?.collect::<rusqlite::Result<Vec<i64>>>().map_err(db_error)?;
            Ok(Detail{quality:quality(&self.db,&memory.input.id)?,memory,provenance,revisions})
        }).collect()
    }
    pub fn deleted_memories(&self, scope: &str, project: &str) -> Result<Vec<serde_json::Value>> {
        identifier(scope)?;
        identifier(project)?;
        let mut q=self.db.prepare("SELECT id,kind,deleted_at FROM memory_tombstones WHERE scope=?1 AND project=?2 ORDER BY deleted_at DESC LIMIT 100").map_err(db_error)?;
        let rows=q.query_map(params![scope,project],|r|Ok(serde_json::json!({"id":r.get::<_,String>(0)?,"kind":r.get::<_,String>(1)?,"deleted_at":r.get::<_,i64>(2)?,"state":"deleted"}))).map_err(db_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(db_error);
        rows
    }
    pub fn undo_consolidation(
        &mut self,
        id: &str,
        scope: &str,
        project: &str,
        expected: i64,
        revision: i64,
        _permit: &ConfigurationPermit,
    ) -> Result<Memory> {
        let current = self.get(id, scope, project)?;
        if current.version != expected {
            return Err("Memory version changed".into());
        }
        let raw: String = self
            .db
            .query_row(
                "SELECT snapshot FROM memory_revisions WHERE memory_id=?1 AND version=?2",
                params![id, revision],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        let prior: Snapshot = serde_json::from_str(&raw).map_err(|_| "Invalid revision")?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let actual: i64 = tx
            .query_row("SELECT version FROM memories WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(db_error)?;
        if actual != expected {
            return Err("Memory changed before restore".into());
        }
        snapshot(&tx, &current)?;
        let conflicts:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memories WHERE id!=?1 AND scope=?2 AND project=?3 AND fact_key=?4 AND state='active')",params![id,scope,project,prior.memory.input.fact_key],|r|r.get(0)).map_err(db_error)?;
        if conflicts {
            return Err("Resolve current conflict before undo".into());
        }
        if prior.memory.state == crate::Status::Active {
            if let Some(q) = &prior.quality {
                let conflict:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memory_quality q JOIN memories n ON n.id=q.memory_id WHERE n.id!=?1 AND n.scope=?2 AND n.project=?3 AND n.state='active' AND q.subject=?4 AND q.property=?5 AND q.value!=?6 AND (?7=0 OR (q.temporal IN ('in_progress','current','completed','cancelled'))=?8))",params![id,scope,project,q.subject,q.property,q.value,q.subject.starts_with("project.") && ["language","database"].contains(&q.property.as_str()),crate::dialogue::execution_track(&q.subject,&q.property,&q.temporal)],|r|r.get(0)).map_err(db_error)?;
                if conflict {
                    return Err("Resolve current topic before restore".into());
                }
            }
        }
        tx.execute("UPDATE memories SET content=?2,fact_key=?3,state=?4,version=version+1,updated_at=?5 WHERE id=?1",params![id,prior.memory.input.content,prior.memory.input.fact_key,types::label(&prior.memory.state),now()]).map_err(db_error)?;
        if let Some(q) = prior.quality {
            put_quality(&tx, id, &q)?
        } else {
            tx.execute("DELETE FROM memory_quality WHERE memory_id=?1", [id])
                .map_err(db_error)?;
        }
        tx.execute("INSERT INTO history(memory_id,record_id,operation,version,at) VALUES(?1,?1,'restore',?2,?3)",params![id,expected+1,now()]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        self.get(id, scope, project)
    }
    /// Rebase only to the exact prior version the reviewer actually saw. The
    /// final replacement still checks versions inside its atomic transaction.
    pub fn resolve_conflict(
        &mut self,
        id: &str,
        scope: &str,
        project: &str,
        expected: i64,
        prior_version: i64,
        _permit: &ConfigurationPermit,
    ) -> Result<Option<Memory>> {
        let current = self.get(id, scope, project)?;
        if current.version != expected
            || !matches!(
                current.state,
                crate::Status::Candidate | crate::Status::Conflict
            )
        {
            return Err("Candidate changed; refresh review".into());
        }
        let mut q = quality(&self.db, id)?.ok_or("No pending replacement")?;
        let prior = self.get(
            q.replaces_id.as_ref().ok_or("No prior fact")?,
            scope,
            project,
        )?;
        if prior.version != prior_version {
            return Err("Prior fact changed; refresh review".into());
        }
        let prior_q = quality(&self.db, &prior.input.id)?.ok_or("Prior topic unavailable")?;
        if prior_q.subject != q.subject || prior_q.property != q.property {
            return Err("Replacement subject mismatch".into());
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let version: i64 = tx
            .query_row("SELECT version FROM memories WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(db_error)?;
        if version != expected {
            return Err("Candidate changed; refresh review".into());
        }
        snapshot(&tx, &current)?;
        q.replaces_version = Some(prior_version);
        put_quality(&tx, id, &q)?;
        tx.commit().map_err(db_error)?;
        self.change(id, scope, project, expected, Action::Approve)
    }
}
fn job_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    Ok(Job {
        id: r.get(0)?,
        state: r.get(1)?,
        attempts: r.get(2)?,
        created_at: r.get(3)?,
        updated_at: r.get(4)?,
        error_code: r.get(5)?,
    })
}
fn ingest(
    tx: &Transaction,
    lease: &Lease,
    c: &Claim,
    policy: &LearningPolicy,
    at: i64,
) -> Result<()> {
    if !crate::source_authority::valid(c) {
        return Err("Original statement authority missing or invalid".into());
    }
    let (conversation_id,message_id):(String,String)=tx.query_row("SELECT conversation_id,message_id FROM extraction_jobs WHERE id=?1",[&lease.id],|r|Ok((r.get(0)?,r.get(1)?))).map_err(db_error)?;
    if !crate::source_authority::bound_to_job(c,&lease.id,&conversation_id,&message_id,&lease.scope,&lease.project) {return Err("Statement authority belongs to another original source".into())}
    if policy.autonomous_preferences
        && !crate::dialogue::project_scope_matches(&c.evidence, &lease.project)
    {
        return Ok(());
    }
    let mut resolved = c.clone();
    if c.subject == "device.unspecified" || c.property == "capacity_update" {
        let old = crate::coverage::old_capacity(&c.evidence);
        let mut q=tx.prepare("SELECT memory_id,subject,value FROM memory_quality q JOIN memories m ON m.id=q.memory_id WHERE m.scope=?1 AND m.project=?2 AND m.state='active' AND q.property='ram_capacity' AND q.subject!='device.unspecified' AND (q.expires_at IS NULL OR q.expires_at>?3) LIMIT 501").map_err(db_error)?;
        let rows = q
            .query_map(params![lease.scope, lease.project, at], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        let matching = rows
            .iter()
            .filter(|(_, subject, value)| {
                if c.subject == "device.unspecified" {
                    old.as_ref() == Some(value)
                } else {
                    subject == &c.subject
                }
            })
            .collect::<Vec<_>>();
        if rows.len() <= 500 && matching.len() == 1 {
            resolved.subject = matching[0].1.clone();
            resolved.property = "ram_capacity".into();
        }
    }
    crate::source_authority::rebind(c, &mut resolved);
    let c = &resolved;
    if let Some((device, _, previous)) = crate::evidence::contextual_ram_replacement(&c.evidence) {
        let expected = format!("{previous}_gb");
        let prior: Option<String> = tx.query_row(
            "SELECT CASE WHEN COUNT(*)=1 THEN MIN(q.value) END
             FROM memories m JOIN memory_quality q ON q.memory_id=m.id
             WHERE m.scope=?1 AND m.project=?2 AND m.state='active'
               AND q.subject=?3 AND q.property='ram_capacity'
               AND (q.expires_at IS NULL OR q.expires_at>?4)",
            params![lease.scope, lease.project, device, at],
            |r| r.get(0),
        ).map_err(db_error)?;
        if prior.as_deref() != Some(expected.as_str()) {
            return Ok(());
        }
    }
    // Order by original source reservation, not by extraction completion time.
    // A slow older job cannot overwrite a later source for the same temporal track.
    let newer: bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memories m JOIN memory_quality q ON q.memory_id=m.id JOIN memory_provenance p ON p.memory_id=m.id JOIN extraction_jobs newer ON newer.id=p.interaction_id JOIN extraction_jobs incoming ON incoming.id=?1 WHERE m.scope=?2 AND m.project=?3 AND m.state IN ('active','candidate','conflict') AND q.subject=?4 AND q.property=?5 AND newer.rowid>incoming.rowid AND (?6=0 OR (q.temporal IN ('in_progress','current','completed','cancelled'))=?7))",params![lease.id,lease.scope,lease.project,c.subject,c.property,c.subject.starts_with("project.") && ["language","database"].contains(&c.property.as_str()),crate::dialogue::execution_track(&c.subject,&c.property,&c.temporal)],|r|r.get(0)).map_err(db_error)?;
    if newer {
        return Ok(());
    }
    let erased:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memory_tombstones WHERE scope=?1 AND project=?2 AND claim_digest=?3)",params![lease.scope,lease.project,c.fingerprint()],|r|r.get(0)).map_err(db_error)?;
    if erased {
        let renewed:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memories m JOIN memory_quality q ON q.memory_id=m.id WHERE m.scope=?1 AND m.project=?2 AND m.state='active' AND q.subject=?3 AND q.property=?4 AND q.value=?5 AND (q.evidence_rule LIKE 'explicit_v3_renewal:%' OR q.evidence_rule LIKE 'llm_v3_grounded:explicit_v3_renewal:%'))",params![lease.scope,lease.project,c.subject,c.property,c.value],|r|r.get(0)).map_err(db_error)?;
        let fresh:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM extraction_jobs j WHERE j.id=?1 AND j.created_at >= COALESCE((SELECT max(deleted_at) FROM memory_tombstones WHERE scope=?2 AND project=?3 AND claim_digest=?4),j.created_at))",params![lease.id,lease.scope,lease.project,c.fingerprint()],|r|r.get(0)).map_err(db_error)?;
        if !(policy.autonomous_preferences && (renewed || c.rule.contains("renewal") && fresh)) {
            return Ok(());
        }
    }
    let mut stmt=tx.prepare(&format!("SELECT {COLS} FROM memories WHERE scope=?1 AND project=?2 AND state!='obsolete' AND (id IN (SELECT memory_id FROM memory_quality WHERE subject=?3 AND property=?4) OR id NOT IN (SELECT memory_id FROM memory_quality)) ORDER BY CASE state WHEN 'active' THEN 0 ELSE 1 END,created_at,id LIMIT 501")).map_err(db_error)?;
    let records = stmt
        .query_map(
            params![lease.scope, lease.project, c.subject, c.property],
            row,
        )
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;
    drop(stmt);
    if records.len() > 500 {
        return Err("Automatic learning review capacity exceeded".into());
    }
    let mut related = Vec::new();
    for memory in records {
        let q = match quality(tx, &memory.input.id)? {
            Some(q) => Some(q),
            None => extract(&memory.input.content)
                .into_iter()
                .find(|p| p.subject == c.subject && p.property == c.property)
                .map(|p| Quality::from_claim(&p, memory.created_at)),
        };
        if let Some(q) = q.filter(|q| {
            q.subject == c.subject
                && q.property == c.property
                && !crate::dialogue::separate_tracks(
                    &c.subject,
                    &c.property,
                    &c.temporal,
                    &q.temporal,
                )
        }) {
            put_quality(tx, &memory.input.id, &q)?;
            related.push((memory, q));
        }
    }
    if let Some((memory, mut q)) = related
        .iter()
        .find(|(_, q)| q.value == c.value && q.temporal == c.temporal)
        .cloned()
    {
        let same_interaction_without_raw:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memory_provenance WHERE memory_id=?1 AND interaction_id=?2) AND NOT EXISTS(SELECT 1 FROM learning_inputs WHERE job_id=?2)",params![memory.input.id,lease.id],|r|r.get(0)).map_err(db_error)?;
        if c.kind == Kind::Preference
            && c.property == "games_tactics"
            && policy.promotes(c)
            && !same_interaction_without_raw
        {
            let before = q.facets.clone();
            q.facets.extend(c.facets.clone());
            q.facets.sort();
            q.facets.dedup();
            if before != q.facets {
                snapshot(tx, &memory)?;
                tx.execute(
                    "UPDATE memories SET content=?2,version=version+1,updated_at=?3 WHERE id=?1",
                    params![memory.input.id, preference_content(&c.value, &q.facets), at],
                )
                .map_err(db_error)?;
                put_quality(tx, &memory.input.id, &q)?;
                tx.execute("INSERT INTO history(memory_id,record_id,operation,version,at) VALUES(?1,?1,'update',?2,?3)",params![memory.input.id,memory.version+1,at]).map_err(db_error)?;
                if matches!(memory.state, crate::Status::Active | crate::Status::Candidate) {
                    crate::source_operations::record(tx, &lease.id, &memory.input.id, memory.version + 1, &types::label(&memory.state), "update")?;
                }
            }
        }
        provenance(tx, &memory.input.id, lease, c, at)?;
        return Ok(());
    }
    let prior = related.first();
    let mut q = Quality::from_claim(c, at);
    if let Some((m, _)) = prior {
        q.replaces_id = Some(m.input.id.clone());
        q.replaces_version = Some(m.version);
    }
    let auto_replace = policy.autonomous_preferences
        && policy.promotes(c)
        && c.correction
        && related.len() == 1
        && prior.is_some_and(|(m, _)| m.state == crate::Status::Active);
    if auto_replace {
        let (old, _) = prior.unwrap();
        snapshot(tx, old)?;
        tx.execute("UPDATE memories SET state='obsolete',version=version+1,updated_at=?2 WHERE id=?1 AND version=?3",params![old.input.id,at,old.version]).map_err(db_error)?;
        tx.execute("INSERT INTO history(memory_id,record_id,operation,version,at) VALUES(?1,?1,'auto_superseded',?2,?3)",params![old.input.id,old.version+1,at]).map_err(db_error)?;
    }
    let state = if prior.is_some() && !auto_replace {
        "conflict"
    } else if policy.promotes(c) {
        "active"
    } else {
        "candidate"
    };
    let id = uuid::Uuid::from_bytes(
        sha2::Sha256::digest(
            serde_json::to_vec(&(&lease.id, c.key(), &c.temporal, &c.value))
                .map_err(|_| "Invalid claim identity")?,
        )[..16]
            .try_into()
            .unwrap(),
    )
    .to_string();
    tx.execute("INSERT OR IGNORE INTO scopes VALUES(?1)", [&lease.scope])
        .map_err(db_error)?;
    let fact_key = if crate::dialogue::execution_track(&c.subject, &c.property, &c.temporal) {
        format!("{}.execution", c.key())
    } else {
        c.key()
    };
    tx.execute(
        "INSERT INTO memories VALUES(?1,?2,?3,?4,?5,?6,?7,'user',?8,1,?9,?9)",
        params![
            id,
            lease.scope,
            lease.project,
            types::label(&c.kind),
            state,
            c.content,
            fact_key,
            lease.id,
            at
        ],
    )
    .map_err(db_error)?;
    put_quality(tx, &id, &q)?;
    provenance(tx, &id, lease, c, at)?;
    if state == "active" && policy.autonomous_preferences {
        tx.execute(
            "INSERT INTO history(memory_id,record_id,operation,version,at) VALUES(?1,?1,?2,1,?3)",
            params![
                id,
                if auto_replace {
                    "auto_correct"
                } else {
                    "auto_promote"
                },
                at
            ],
        )
        .map_err(db_error)?;
    }
    tx.execute(
        "INSERT INTO history(memory_id,record_id,operation,version,at) VALUES(?1,?1,'create',1,?2)",
        params![id, at],
    )
    .map_err(db_error)?;
    if state != "conflict" {
        crate::source_operations::record(tx, &lease.id, &id, 1, state, if auto_replace { "update" } else { "create" })?;
    }
    Ok(())
}
fn provenance(tx: &Transaction, id: &str, lease: &Lease, c: &Claim, at: i64) -> Result<()> {
    let previous: Option<String> = tx
        .query_row(
            "SELECT evidence FROM memory_provenance WHERE memory_id=?1 AND interaction_id=?2",
            params![id, lease.id],
            |r| r.get(0),
        )
        .optional()
        .map_err(db_error)?;
    if let Some(previous) = previous.filter(|old| !old.contains(&c.evidence)) {
        let source: Option<String> = tx
            .query_row(
                "SELECT user_text FROM learning_inputs WHERE job_id=?1",
                [&lease.id],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        if let Some(source) = source {
            if let (Some(old_start), Some(new_start)) =
                (source.find(&previous), source.find(&c.evidence))
            {
                let begin = old_start.min(new_start);
                let end = (old_start + previous.len()).max(new_start + c.evidence.len());
                // One exact contiguous source span preserves both clauses under
                // the existing one-row-per-interaction provenance schema.
                tx.execute("UPDATE memory_provenance SET evidence=?3 WHERE memory_id=?1 AND interaction_id=?2",params![id,lease.id,&source[begin..end]]).map_err(db_error)?;
                return Ok(());
            }
        }
    }
    tx.execute(
        "INSERT OR IGNORE INTO memory_provenance VALUES(?1,?2,?3,?4)",
        params![id, lease.id, c.evidence, at],
    )
    .map_err(db_error)?;
    Ok(())
}
