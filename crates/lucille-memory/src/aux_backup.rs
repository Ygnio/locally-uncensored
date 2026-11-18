//! Fixed-schema, parameterized backup of derived learning state. No imported SQL.
use crate::{
    db_error, identifier,
    intelligent::{quality, Claim, LearningPolicy},
    semantic::EmbeddingSpace,
    Result,
};
use rusqlite::{
    params_from_iter,
    types::{Value, ValueRef},
    Connection, Transaction,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
const TABLES: &[(&str, usize)] = &[
    ("learning_policy", 2),
    ("extraction_jobs", 15),
    ("learning_inputs", 3),
    ("memory_quality", 13),
    ("memory_provenance", 4),
    ("memory_revisions", 4),
    ("memory_tombstones", 6),
    ("embedding_spaces", 3),
    ("memory_vectors", 4),
    ("memory_source_operations", 4),
    ("memory_source_operation_records", 5),
];
#[derive(Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AuxBackup {
    tables: BTreeMap<String, Vec<Vec<serde_json::Value>>>,
}
pub(crate) fn export(db: &Connection) -> Result<AuxBackup> {
    let mut tables = BTreeMap::new();
    for &(table, columns) in TABLES {
        // Preserve original source ordering across backup/restore. Other tables
        // remain sorted by stable keys for deterministic exports.
        let order = if table == "extraction_jobs" {
            "rowid"
        } else {
            "1,2"
        };
        let filter = if table.starts_with("memory_source_operation") {
            "WHERE source_id IN (SELECT id FROM extraction_jobs WHERE state='done')"
        } else {
            ""
        };
        let mut q = db
            .prepare(&format!("SELECT * FROM {table} {filter} ORDER BY {order}"))
            .map_err(db_error)?;
        let mut rows = q.query([]).map_err(db_error)?;
        let mut out = Vec::new();
        while let Some(row) = rows.next().map_err(db_error)? {
            let mut values = Vec::new();
            for col in 0..columns {
                let value = match row.get_ref(col).map_err(db_error)? {
                    ValueRef::Null => serde_json::Value::Null,
                    ValueRef::Integer(v) => v.into(),
                    ValueRef::Real(v) => serde_json::json!(v),
                    ValueRef::Text(v) => serde_json::Value::String(
                        String::from_utf8(v.to_vec()).map_err(|_| "Invalid UTF-8 backup")?,
                    ),
                    _ => return Err("Unexpected binary learning state".into()),
                };
                values.push(value)
            }
            out.push(values);
            if out.len() > 10000 {
                return Err("Learning backup exceeds row ceiling".into());
            }
        }
        if filter.is_empty() || !out.is_empty() {
            tables.insert(table.to_string(), out);
        }
    }
    Ok(AuxBackup { tables })
}
pub(crate) fn clear(tx: &Transaction) -> Result<()> {
    for table in [
        "learning_inputs",
        "memory_vectors",
        "embedding_spaces",
        "memory_provenance",
        "memory_revisions",
        "memory_quality",
        "memory_tombstones",
        "extraction_jobs",
        "learning_policy",
    ] {
        tx.execute(&format!("DELETE FROM {table}"), [])
            .map_err(db_error)?;
    }
    Ok(())
}
pub(crate) fn restore(tx: &Transaction, backup: &AuxBackup) -> Result<()> {
    if backup
        .tables
        .keys()
        .any(|t| !TABLES.iter().any(|(name, _)| *name == t))
    {
        return Err("Unknown backup table".into());
    }
    for &(table, columns) in TABLES {
        if let Some(rows) = backup.tables.get(table) {
            if rows.len() > 10000 {
                return Err("Too many learning records".into());
            }
            for row in rows {
                if row.len() != columns {
                    return Err("Invalid learning record width".into());
                }
                let values = row
                    .iter()
                    .map(|v| match v {
                        serde_json::Value::Null => Ok(Value::Null),
                        serde_json::Value::String(s) if s.len() <= 65536 => {
                            Ok(Value::Text(s.clone()))
                        }
                        serde_json::Value::Number(n) => {
                            if let Some(v) = n.as_i64() {
                                Ok(Value::Integer(v))
                            } else if let Some(v) = n.as_f64().filter(|v| v.is_finite()) {
                                Ok(Value::Real(v))
                            } else {
                                Err("Invalid backup number")
                            }
                        }
                        _ => Err("Invalid backup scalar"),
                    })
                    .collect::<std::result::Result<Vec<_>, _>>()?;
                tx.execute(
                    &format!(
                        "INSERT INTO {table} VALUES({})",
                        vec!["?"; columns].join(",")
                    ),
                    params_from_iter(values),
                )
                .map_err(db_error)?;
            }
        }
    }
    let invalid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM learning_inputs i JOIN extraction_jobs j ON j.id=i.job_id WHERE j.state NOT IN ('queued','running','retry'))",[],|r|r.get(0)).map_err(db_error)?;
    if invalid {
        return Err("Terminal input retained in backup".into());
    }
    {
        let mut q = tx
            .prepare("SELECT user_text FROM learning_inputs")
            .map_err(db_error)?;
        let rows = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_error)?;
        for row in rows {
            if !crate::llm::eligible_source(&row.map_err(db_error)?) {
                return Err("Ineligible source backup".into());
            }
        }
    }
    let policy: Option<String> = tx
        .query_row("SELECT json FROM learning_policy WHERE id=1", [], |r| {
            r.get(0)
        })
        .optional()
        .map_err(db_error)?;
    if let Some(raw) = policy {
        let p: LearningPolicy = serde_json::from_str(&raw).map_err(|_| "Invalid policy backup")?;
        p.validate()?;
    }
    let mut q=tx.prepare("SELECT id,conversation_id,message_id,scope,project,mode,state,evidence_json,attempts FROM extraction_jobs").map_err(db_error)?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, String>(5)?,
                r.get::<_, String>(6)?,
                r.get::<_, String>(7)?,
                r.get::<_, i64>(8)?,
            ))
        })
        .map_err(db_error)?;
    for row in rows {
        let (id, conversation, message, scope, project, mode, state, raw, attempts) =
            row.map_err(db_error)?;
        for key in [&id, &conversation, &message, &scope, &project] {
            identifier(key)?
        }
        let _: crate::Mode =
            serde_json::from_value(serde_json::json!(mode)).map_err(|_| "Invalid learning mode")?;
        let claims: Vec<Claim> =
            serde_json::from_str(&raw).map_err(|_| "Invalid evidence backup")?;
        if claims.len() > 4
            || claims.iter().any(|c| {
                !crate::source_authority::independently_interpreted(c,&project)
                    || !crate::source_authority::bound_to_job(c,&id,&conversation,&message,&scope,&project)
            })
            || (!(0..=3).contains(&attempts))
            || (["done", "failed", "cancelled"].contains(&state.as_str()) && !claims.is_empty())
        {
            return Err("Unverified queue backup".into());
        }
    }
    drop(q);
    let ids = {
        let mut q = tx
            .prepare("SELECT memory_id FROM memory_quality")
            .map_err(db_error)?;
        let rows = q
            .query_map([], |r| r.get::<_, String>(0))
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        rows
    };
    for id in ids {
        if let Some(q) = quality(tx, &id)? {
            identifier(&q.subject)?;
            identifier(&q.property)?;
            if q.value.len() > 1000
                || q.facets.len() > 8
                || q.facets.iter().any(|f| f.len() > 32)
                || !q.confidence.is_finite()
                || !q.relevance.is_finite()
            {
                return Err("Invalid quality backup".into());
            }
        }
    }
    let mut spaces = BTreeMap::new();
    let mut q = tx
        .prepare("SELECT id,spec_json FROM embedding_spaces")
        .map_err(db_error)?;
    let rows = q
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(db_error)?;
    for row in rows {
        let (id, raw) = row.map_err(db_error)?;
        let spec: EmbeddingSpace =
            serde_json::from_str(&raw).map_err(|_| "Invalid embedding backup")?;
        if spec.id()? != id {
            return Err("Embedding fingerprint backup mismatch".into());
        }
        spaces.insert(id, spec);
    }
    drop(q);
    let mut q = tx
        .prepare("SELECT space_id,vector_json FROM memory_vectors")
        .map_err(db_error)?;
    let rows = q
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(db_error)?;
    for row in rows {
        let (id, raw) = row.map_err(db_error)?;
        let vector: Vec<f32> = serde_json::from_str(&raw).map_err(|_| "Invalid vector backup")?;
        spaces
            .get(&id)
            .ok_or("Unknown vector space")?
            .check_vector(&vector)?;
    }
    drop(q);
    crate::source_operations::validate_restored(tx)?;
    // Imported leases are not authority in this process. Retain attempt bounds.
    tx.execute("UPDATE extraction_jobs SET state=CASE WHEN attempts>=3 THEN 'failed' ELSE 'retry' END,evidence_json=CASE WHEN attempts>=3 THEN '[]' ELSE evidence_json END,lease_token=NULL,lease_until=NULL WHERE state='running'",[]).map_err(db_error)?;
    Ok(())
}
use rusqlite::OptionalExtension;
