//! Operation identity and record versions are captured in the writer transaction.
//! Backups carry only completed receipts, with validation before commit.
use crate::{db_error, intelligent::Interaction, turn_sources, Result, TurnRequest};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use serde_json::{json, Value};

pub(crate) fn reserve(db: &Connection, id: &str, property: Option<&str>) -> Result<()> {
    db.execute(
        "INSERT OR IGNORE INTO memory_source_operations(source_id,identity_version,action,property)
         SELECT ?1,version,?2,?3 FROM identity_profile WHERE id=1",
        params![
            id,
            if property.is_some() {
                "forget"
            } else {
                "learn"
            },
            property
        ],
    )
    .map_err(db_error)?;
    Ok(())
}

pub(crate) fn record(
    db: &Connection,
    source_id: &str,
    record_id: &str,
    version: i64,
    state: &str,
    effect: &str,
) -> Result<()> {
    db.execute(
        "INSERT INTO memory_source_operation_records(source_id,record_id,version,state,effect)
         SELECT source_id,?2,?3,?4,?5 FROM memory_source_operations WHERE source_id=?1",
        params![source_id, record_id, version, state, effect],
    )
    .map_err(db_error)?;
    Ok(())
}

struct BoundOperation {
    version: i64,
    action: String,
    property: Option<String>,
    state: String,
    deferred: bool,
}

fn bound(db: &Connection, id: &str, input: &Interaction) -> Result<Option<BoundOperation>> {
    db.query_row(
        "SELECT o.identity_version,o.action,o.property,j.state,j.lease_token IS NOT NULL
         FROM memory_source_operations o JOIN extraction_jobs j ON j.id=o.source_id
         JOIN identity_profile i ON i.id=1 AND i.version=o.identity_version
         WHERE j.id=?1 AND j.conversation_id=?2 AND j.message_id=?3 AND j.scope=?4 AND j.project=?5 AND j.mode=?6",
        params![id, input.conversation_id, input.message_id, input.scope, input.project, crate::types::label(&input.mode)],
        |r| Ok(BoundOperation { version:r.get(0)?, action:r.get(1)?, property:r.get(2)?, state:r.get(3)?, deferred:r.get(4)? }),
    )
    .optional()
    .map_err(db_error)
}

fn envelope(id: &str, input: &Interaction, operation: &BoundOperation, state: &str) -> Value {
    json!({"id":id,"action":operation.action,"state":state,
           "profile":"lucille","identity_version":operation.version,"mode":crate::types::label(&input.mode),
           "scope":input.scope,"project":input.project,"conversation_id":input.conversation_id,"source_message_id":input.message_id})
}

#[derive(Serialize)]
struct LearningRecord {
    id: String,
    version: i64,
    state: String,
    effect: String,
    entity: String,
    property: String,
    value: String,
}

pub(crate) fn learning(
    db: &Connection,
    req: &TurnRequest,
    source: &turn_sources::SourceProof,
) -> Result<Option<Value>> {
    let id = turn_sources::validate(req, source)?;
    if source.job_id.as_deref() != Some(&id) {
        return Ok(None);
    }
    let input = &source.interaction;
    let Some(operation) =
        bound(db, &id, input)?.filter(|o| o.state == "done" && o.action == "learn")
    else {
        return Ok(None);
    };
    let mut stmt = db.prepare(
        "SELECT r.record_id,r.version,r.state,r.effect,q.subject,q.property,q.value
         FROM memory_source_operation_records r JOIN memories m ON m.id=r.record_id AND m.version=r.version AND m.state=r.state
         JOIN memory_quality q ON q.memory_id=m.id JOIN memory_provenance p ON p.memory_id=m.id AND p.interaction_id=r.source_id
         WHERE r.source_id=?1 AND m.scope=?2 AND m.project=?3 AND (q.expires_at IS NULL OR q.expires_at>?4) ORDER BY r.record_id LIMIT 17",
    ).map_err(db_error)?;
    let records = stmt
        .query_map(params![id, req.scope, req.project, crate::now()], |r| {
            Ok(LearningRecord {
                id: r.get(0)?,
                version: r.get(1)?,
                state: r.get(2)?,
                effect: r.get(3)?,
                entity: r.get(4)?,
                property: r.get(5)?,
                value: r.get(6)?,
            })
        })
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;
    if records.is_empty() || records.len() > 16 {
        return Ok(None);
    }
    let mut receipt = envelope(&id, input, &operation, "done");
    receipt["records"] = json!(records);
    Ok(Some(receipt))
}

pub(crate) fn forget(db: &Connection, input: &Interaction) -> Result<Option<Value>> {
    let id = turn_sources::id(input)?;
    let Some(operation) =
        bound(db, &id, input)?.filter(|o| o.state == "done" && o.action == "forget")
    else {
        return Ok(None);
    };
    if operation.property.as_deref() != crate::autonomous::forget_property(&input.user_text) {
        return Ok(None);
    }
    let op: Option<String> = db.query_row(
        "SELECT operation FROM history WHERE record_id=?1 AND operation IN ('forget_applied','forget_empty') ORDER BY sequence DESC LIMIT 1",
        [turn_sources::receipt_id(&id)?], |r| r.get(0),
    ).optional().map_err(db_error)?;
    let Some(op) = op else { return Ok(None) };
    let mut stmt = db.prepare(
        "SELECT r.record_id,r.version,EXISTS(SELECT 1 FROM memory_tombstones t WHERE t.id=r.record_id AND t.scope=?2 AND t.project=?3)
         AND NOT EXISTS(SELECT 1 FROM memories m WHERE m.id=r.record_id)
         FROM memory_source_operation_records r WHERE r.source_id=?1 AND r.state='deleted' AND r.effect='delete' ORDER BY r.record_id LIMIT 65",
    ).map_err(db_error)?;
    let targets = stmt
        .query_map(params![id, input.scope, input.project], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })
        .map_err(db_error)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(db_error)?;
    let had_records = op == "forget_applied";
    if targets.len() > 64 || targets.iter().any(|row| !row.2) || had_records == targets.is_empty() {
        return Ok(None);
    }
    let mut receipt = envelope(&id, input, &operation, "done");
    receipt["property"] = json!(operation.property);
    receipt["had_records"] = json!(had_records);
    receipt["records"] = json!(targets.into_iter().map(|(id,version,_)| json!({"id":id,"version":version,"state":"deleted","effect":"delete"})).collect::<Vec<_>>());
    Ok(Some(receipt))
}

pub(crate) fn validate_restored(db: &Connection) -> Result<()> {
    let invalid_operation: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM memory_source_operations o JOIN extraction_jobs j ON j.id=o.source_id
         WHERE j.state!='done' OR j.lease_token IS NOT NULL
         OR o.identity_version>COALESCE((SELECT version FROM identity_profile WHERE id=1),0))",
        [], |r| r.get(0),
    ).map_err(db_error)?;
    let invalid_record: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM memory_source_operation_records r
         JOIN memory_source_operations o ON o.source_id=r.source_id
         JOIN extraction_jobs j ON j.id=r.source_id
         WHERE NOT EXISTS(SELECT 1 FROM history h WHERE h.record_id=r.record_id AND h.version=r.version
            AND ((r.effect='delete' AND h.operation='delete') OR (r.effect='create' AND h.operation='create')
              OR (r.effect='update' AND h.operation IN ('update','auto_correct'))))
         OR (o.action='forget' AND (r.effect!='delete' OR r.state!='deleted' OR NOT EXISTS(
            SELECT 1 FROM memory_tombstones t WHERE t.id=r.record_id AND t.scope=j.scope AND t.project=j.project)))
         OR (o.action='learn' AND (r.effect='delete' OR r.state='deleted')))",
        [], |r| r.get(0),
    ).map_err(db_error)?;
    if invalid_operation || invalid_record {
        return Err("Invalid completed memory operation backup".into());
    }
    let mut stmt = db.prepare(
        "SELECT source_id,property,(SELECT count(*) FROM memory_source_operation_records r WHERE r.source_id=o.source_id)
         FROM memory_source_operations o WHERE o.action='forget'",
    ).map_err(db_error)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })
        .map_err(db_error)?;
    for row in rows {
        let (id, property, records) = row.map_err(db_error)?;
        crate::identifier(&property)?;
        // Source IDs are native SHA-256 values; do not pass arbitrary bytes to UUID derivation.
        if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) || records > 64 {
            return Err("Invalid forget receipt backup".into());
        }
        let op = if records == 0 {
            "forget_empty"
        } else {
            "forget_applied"
        };
        let exists: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM history WHERE record_id=?1 AND operation=?2 AND version=1)",
            params![turn_sources::receipt_id(&id)?,op], |r| r.get(0),
        ).map_err(db_error)?;
        if !exists {
            return Err("Invalid forget receipt history".into());
        }
    }
    Ok(())
}

pub(crate) fn status(
    db: &Connection,
    req: &TurnRequest,
    source: &turn_sources::SourceProof,
) -> Result<Option<Value>> {
    let id = turn_sources::validate(req, source)?;
    if source.job_id.as_deref() != Some(&id) {
        return Ok(None);
    }
    let input = &source.interaction;
    Ok(bound(db, &id, input)?.map(|operation| {
        let stage = match operation.state.as_str() {
            "queued" if operation.deferred => "source_persisted",
            "queued" | "running" | "retry" => "processing",
            "done" => "completed",
            "failed" => "failed",
            "cancelled" => "cancelled",
            _ => "received",
        };
        envelope(&id, input, &operation, stage)
    }))
}
