//! A user source is reserved before generation, published only after durable output.
//! Existing queued jobs carry a temporary native-process lease, never a new writer.
use crate::{db_error, extraction::digest, intelligent::Interaction, Result, Store, TurnRequest};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

pub(crate) fn owner() -> &'static str {
    static OWNER: std::sync::LazyLock<String> =
        std::sync::LazyLock::new(|| uuid::Uuid::new_v4().to_string());
    OWNER.as_str()
}
pub(crate) fn id(input: &Interaction) -> Result<String> {
    for s in [
        &input.conversation_id,
        &input.message_id,
        &input.scope,
        &input.project,
    ] {
        crate::identifier(s)?;
    }
    if input.role != "user" || input.user_text.len() > 32000 {
        return Err("Invalid user source".into());
    }
    Ok(digest(
        &serde_json::to_string(&(
            &input.conversation_id,
            &input.message_id,
            &input.scope,
            &input.project,
            digest(&input.user_text),
        ))
        .map_err(|_| "Invalid source")?,
    ))
}
pub(crate) fn receipt_id(id: &str) -> Result<String> {
    uuid::Uuid::parse_str(&id[..32])
        .map(|v| v.to_string())
        .map_err(|_| "Invalid receipt identifier".into())
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProof {
    pub interaction: Interaction,
    pub job_id: Option<String>,
}
pub(crate) fn validate(req: &TurnRequest, source: &SourceProof) -> Result<String> {
    let i = &source.interaction;
    let id = id(i)?;
    if i.conversation_id != req.conversation_id
        || i.scope != req.scope
        || i.project != req.project
        || crate::types::label(&i.mode) != crate::types::label(&req.mode)
        || i.user_text != req.query
        || source.job_id.as_ref().is_some_and(|v| v != &id)
    {
        return Err("User source binding mismatch".into());
    }
    Ok(id)
}
pub(crate) fn receipt(db: &Connection, input: &Interaction) -> Result<Option<serde_json::Value>> {
    let Some(property) = crate::autonomous::forget_property(&input.user_text) else {
        return Ok(None);
    };
    let id = id(input)?;
    let op:Option<String>=db.query_row("SELECT h.operation FROM history h WHERE h.record_id=?1 AND h.operation IN ('forget_applied','forget_empty') AND EXISTS(SELECT 1 FROM extraction_jobs j WHERE j.id=?2 AND j.state='done') ORDER BY h.sequence DESC LIMIT 1",params![receipt_id(&id)?,id],|r|r.get(0)).optional().map_err(db_error)?;
    Ok(op.map(|op|serde_json::json!({"id":id,"action":"forget","property":property,"state":"done","had_records":op=="forget_applied","scope":input.scope,"project":input.project,"conversation_id":input.conversation_id,"source_message_id":input.message_id})))
}
pub(crate) fn statements(
    db: &Connection,
    req: &TurnRequest,
    source: &SourceProof,
) -> Result<Vec<crate::context_facts::ContextFact>> {
    let id = validate(req, source)?;
    let state: Option<(String, Option<String>, Option<i64>)> = db
        .query_row(
            "SELECT state,lease_token,lease_until FROM extraction_jobs WHERE id=?1",
            [&id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()
        .map_err(db_error)?;
    let allowed = match state {
        None => source.job_id.is_none(),
        Some((state, token, until)) => {
            ["queued", "retry", "running"].contains(&state.as_str())
                && (state != "queued"
                    || token.is_none()
                    || (token.as_deref() == Some(owner())
                        && until.is_some_and(|t| t > crate::now())))
        }
    };
    Ok(if allowed {
        crate::context_facts::statements(req)
    } else {
        Vec::new()
    })
}
impl Store {
    pub(crate) fn recover_pending_user_turns(&mut self, at: i64) -> Result<()> {
        self.db.execute("UPDATE extraction_jobs SET state='cancelled',evidence_json='[]',error_code='interrupted_user_turn',lease_token=NULL,lease_until=NULL,updated_at=?1 WHERE state='queued' AND lease_token IS NOT NULL AND (lease_token!=?2 OR lease_until<=?1)",params![at,owner()]).map_err(db_error)?;
        self.db.execute("DELETE FROM learning_inputs WHERE job_id IN (SELECT id FROM extraction_jobs WHERE state='cancelled')",[]).map_err(db_error)?;
        Ok(())
    }
    pub fn begin_user_turn(&mut self, input: Interaction) -> Result<serde_json::Value> {
        let job = self.prepare_user_turn(input.clone())?;
        let proof = SourceProof {
            job_id: job.get("id").and_then(|v| v.as_str()).map(str::to_owned),
            interaction: input.clone(),
        };
        let req = TurnRequest {
            conversation_id: input.conversation_id.clone(),
            message_id: input.message_id.clone(),
            scope: input.scope.clone(),
            project: input.project.clone(),
            mode: input.mode.clone(),
            query: input.user_text.clone(),
            token_budget: 8192,
        };
        Ok(
            serde_json::json!({"job":job,"proof":proof,"current_statements":statements(&self.db,&req,&proof)?,"directive_receipt":receipt(&self.db,&input)?}),
        )
    }
    pub fn publish_user_turn(&mut self, source: SourceProof) -> Result<serde_json::Value> {
        let id = id(&source.interaction)?;
        if source.job_id.as_deref() != Some(&id) {
            return Ok(serde_json::json!({"state":"ignored","reason":"no_prepared_source"}));
        }
        if self.learning_job(&id)?.is_none() {
            return Err("Prepared source no longer exists".into());
        }
        self.enqueue_learning(source.interaction)
    }
    pub fn cancel_user_turn(&mut self, source: SourceProof) -> Result<bool> {
        let id = id(&source.interaction)?;
        if source.job_id.as_ref().is_some_and(|value| value != &id) {
            return Ok(false);
        }
        // Cancellation may arrive before the prepare response reaches the UI.
        // A missing proof can cancel only this process's reserved source.
        if source.job_id.is_none() {
            let owned: bool = self.db.query_row("SELECT EXISTS(SELECT 1 FROM extraction_jobs WHERE id=?1 AND state='queued' AND lease_token=?2)", params![id,owner()], |r| r.get(0)).map_err(db_error)?;
            if !owned {
                return Ok(false);
            }
        }
        self.cancel_learning(
            &id,
            &crate::ConfigurationPermit::explicit_settings_action(true)?,
        )
    }
}
