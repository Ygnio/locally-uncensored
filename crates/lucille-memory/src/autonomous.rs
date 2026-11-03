//! Explicit, scoped user forgetting and derived-index work. SQLite remains canonical.
use crate::{
    db_error,
    extraction::{digest, fold},
    intelligent::{before_change, Interaction},
    row, types, Action, Result, Store, COLS,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
impl Store {
    pub(crate) fn forget_user_directive(
        &mut self,
        input: &Interaction,
    ) -> Result<Option<serde_json::Value>> {
        let s = fold(input.user_text.trim().trim_end_matches(['.', '!']));
        let property = match s.as_str() {
            "esqueca minha preferencia por cafe"
            | "esqueca minha preferencia de cafe"
            | "forget my coffee preference" => "coffee_sugar",
            "esqueca meu idioma preferido" | "forget my preferred language" => "response_language",
            "esqueca minha preferencia de tema" | "forget my theme preference" => "interface_theme",
            _ => return Ok(None),
        };
        if input.role != "user" || !self.learning_policy()?.autonomous_preferences {
            return Ok(None);
        }
        let id = digest(
            &serde_json::to_string(&(
                &input.conversation_id,
                &input.message_id,
                &input.scope,
                &input.project,
                digest(&input.user_text),
            ))
            .map_err(|_| "Invalid directive")?,
        );
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        if tx
            .query_row("SELECT 1 FROM extraction_jobs WHERE id=?1", [&id], |r| {
                r.get::<_, i64>(0)
            })
            .optional()
            .map_err(db_error)?
            .is_some()
        {
            return Ok(Some(
                serde_json::json!({"state":"done","id":id,"reason":"directive_already_processed"}),
            ));
        }
        let at = crate::now();
        let memories = {
            let mut q=tx.prepare(&format!("SELECT {} FROM memories WHERE scope=?1 AND project=?2 AND id IN (SELECT memory_id FROM memory_quality WHERE subject='user' AND property=?3)",COLS)).map_err(db_error)?;
            let rows = q
                .query_map(params![input.scope, input.project, property], row)
                .map_err(db_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(db_error)?;
            rows
        };
        if memories.len() > 64 {
            return Err("Forget review capacity exceeded".into());
        }
        let jobs = {
            let mut q=tx.prepare("SELECT j.id,COALESCE(i.user_text,j.evidence_json) FROM extraction_jobs j LEFT JOIN learning_inputs i ON i.job_id=j.id WHERE j.scope=?1 AND j.project=?2 AND j.state IN ('queued','retry','running') LIMIT 65").map_err(db_error)?;
            let rows = q
                .query_map(params![input.scope, input.project], |r| {
                    Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
                })
                .map_err(db_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(db_error)?;
            rows
        };
        for (job, text) in jobs {
            let text = fold(&text);
            let relevant = if property == "coffee_sugar" {
                text.contains("cafe") || text.contains("coffee")
            } else if property == "response_language" {
                text.contains("portugues") || text.contains("language")
            } else {
                text.contains("tema") || text.contains("theme") || text.contains("interface")
            };
            if relevant {
                tx.execute("UPDATE extraction_jobs SET state='cancelled',evidence_json='[]',lease_token=NULL,lease_until=NULL,error_code='user_forgot_topic',updated_at=?2 WHERE id=?1",params![job,at]).map_err(db_error)?;
            }
        }
        for memory in &memories {
            before_change(&tx, memory, &Action::Delete)?;
            tx.execute("DELETE FROM memories WHERE id=?1", [&memory.input.id])
                .map_err(db_error)?;
            tx.execute("INSERT INTO history(memory_id,record_id,operation,version,at) VALUES(NULL,?1,'delete',?2,?3)",params![memory.input.id,memory.version+1,at]).map_err(db_error)?;
        }
        tx.execute("INSERT INTO extraction_jobs(id,conversation_id,message_id,scope,project,mode,state,evidence_json,created_at,updated_at,next_at) VALUES(?1,?2,?3,?4,?5,?6,'done','[]',?7,?7,?7)",params![id,input.conversation_id,input.message_id,input.scope,input.project,types::label(&input.mode),at]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(Some(
            serde_json::json!({"id":id,"state":"done","reason":"user_forgot_topic","deleted":memories.len()}),
        ))
    }
    /// Pending derived work is discovered from canonical versions, so it survives
    /// crashes without a second authoritative queue or a model-controlled writer.
    pub fn pending_embedding_records(
        &self,
        scope: &str,
        project: &str,
        limit: usize,
    ) -> Result<serde_json::Value> {
        crate::identifier(scope)?;
        crate::identifier(project)?;
        if !(1..=64).contains(&limit) {
            return Err("Index batch ceiling".into());
        }
        let policy = self.learning_policy()?;
        if !policy.enabled || !policy.automatic_indexing {
            return Ok(serde_json::json!([]));
        }
        let mut q=self.db.prepare("SELECT m.id,m.version FROM memories m LEFT JOIN memory_vectors v ON v.memory_id=m.id LEFT JOIN memory_quality q ON q.memory_id=m.id WHERE m.scope=?1 AND m.project=?2 AND m.state='active' AND (q.expires_at IS NULL OR q.expires_at>?3) AND (v.memory_id IS NULL OR v.memory_version!=m.version OR v.space_id!=COALESCE((SELECT id FROM embedding_spaces WHERE active=1),'')) ORDER BY m.updated_at,m.id LIMIT ?4").map_err(db_error)?;
        let rows = q
            .query_map(params![scope, project, crate::now(), limit as i64], |r| {
                Ok(serde_json::json!({"id":r.get::<_,String>(0)?,"version":r.get::<_,i64>(1)?}))
            })
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        Ok(serde_json::json!(rows))
    }
}

impl Store {
    pub fn learning_metrics(&self, scope: &str, project: &str) -> Result<serde_json::Value> {
        crate::identifier(scope)?;
        crate::identifier(project)?;
        let details = self.memory_details(scope, project)?;
        let counts = |sql: &str| -> Result<Vec<serde_json::Value>> {
            let mut q = self.db.prepare(sql).map_err(db_error)?;
            let rows=q.query_map(params![scope,project],|r|Ok(serde_json::json!({"state":r.get::<_,String>(0)?,"count":r.get::<_,i64>(1)?}))).map_err(db_error)?.collect::<rusqlite::Result<Vec<_>>>().map_err(db_error)?;
            Ok(rows)
        };
        let jobs=counts("SELECT state,count(*) FROM extraction_jobs WHERE scope=?1 AND project=?2 GROUP BY state")?;
        let memories = counts(
            "SELECT state,count(*) FROM memories WHERE scope=?1 AND project=?2 GROUP BY state",
        )?;
        let events=counts("SELECT h.operation,count(*) FROM history h JOIN memories m ON m.id=h.record_id WHERE m.scope=?1 AND m.project=?2 GROUP BY h.operation")?;
        let vectors:i64=self.db.query_row("SELECT count(*) FROM memory_vectors v JOIN memories m ON m.id=v.memory_id WHERE m.scope=?1 AND m.project=?2 AND m.state='active' AND v.memory_version=m.version",params![scope,project],|r|r.get(0)).map_err(db_error)?;
        let pending:i64=self.db.query_row("SELECT count(*) FROM memories m LEFT JOIN memory_vectors v ON v.memory_id=m.id LEFT JOIN memory_quality q ON q.memory_id=m.id WHERE m.scope=?1 AND m.project=?2 AND m.state='active' AND (q.expires_at IS NULL OR q.expires_at>strftime('%s','now')) AND (v.memory_id IS NULL OR v.memory_version!=m.version OR v.space_id!=COALESCE((SELECT id FROM embedding_spaces WHERE active=1),''))",params![scope,project],|r|r.get(0)).map_err(db_error)?;
        let bytes:i64=self.db.query_row("SELECT COALESCE(sum(length(v.vector_json)),0) FROM memory_vectors v JOIN memories m ON m.id=v.memory_id WHERE m.scope=?1 AND m.project=?2",params![scope,project],|r|r.get(0)).map_err(db_error)?;
        let pages: i64 = self
            .db
            .query_row("PRAGMA page_count", [], |r| r.get(0))
            .map_err(db_error)?;
        let page_size: i64 = self
            .db
            .query_row("PRAGMA page_size", [], |r| r.get(0))
            .map_err(db_error)?;
        Ok(
            serde_json::json!({"scope":scope,"project":project,"jobs":jobs,"memories":memories,"history_events":events,"current_vectors":vectors,"pending_vectors":pending,"vector_json_bytes":bytes,"sqlite_main_page_bytes":pages*page_size,"provenance_rows":details.iter().map(|d|d.provenance.len()).sum::<usize>()}),
        )
    }
}
