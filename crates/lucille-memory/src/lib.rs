//! Canonical, opt-in SQLite memory. No inference, networking, process control or personal-data discovery.
pub mod isolation;
mod types;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
pub use types::*;

pub type Result<T> = std::result::Result<T, String>;
fn db_error(_: rusqlite::Error) -> String {
    "Lucille database operation failed".into()
}
fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
fn text(s: &str, max: usize) -> Result<()> {
    if s.trim().is_empty() || s.len() > max || s.contains('\0') {
        Err("Invalid argument".into())
    } else {
        Ok(())
    }
}
fn identifier(s: &str) -> Result<()> {
    text(s, 128)?;
    if s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "-_.:".contains(c))
    {
        Ok(())
    } else {
        Err("Invalid identifier".into())
    }
}
impl Candidate {
    fn validate(&self) -> Result<()> {
        uuid::Uuid::parse_str(&self.id).map_err(|_| "Invalid memory ID")?;
        identifier(&self.scope)?;
        identifier(&self.project)?;
        identifier(&self.fact_key)?;
        identifier(&self.source_id)?;
        text(&self.content, 16000)
    }
}
fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<Memory> {
    Ok(Memory {
        input: Candidate {
            id: r.get(0)?,
            scope: r.get(1)?,
            project: r.get(2)?,
            kind: types::parse(r.get(3)?)?,
            content: r.get(5)?,
            fact_key: r.get(6)?,
            source: types::parse(r.get(7)?)?,
            source_id: r.get(8)?,
        },
        state: types::parse(r.get(4)?)?,
        version: r.get(9)?,
        created_at: r.get(10)?,
        updated_at: r.get(11)?,
    })
}
const COLS: &str =
    "id,scope,project,kind,state,content,fact_key,source,source_id,version,created_at,updated_at";

/// The host owns this capability. It is never accepted in a model/tool payload.
pub struct ConfigurationPermit(());
impl ConfigurationPermit {
    /// Called only by an explicit settings action in the trusted host UI.
    pub fn explicit_settings_action(confirmed: bool) -> Result<Self> {
        if confirmed {
            Ok(Self(()))
        } else {
            Err("Explicit configuration authorization required".into())
        }
    }
}

pub struct Store {
    db: Connection,
}
impl Store {
    /// Path is supplied by the trusted host; the IPC API never accepts a path or SQL.
    pub fn open(path: &Path) -> Result<Self> {
        let mut db = Connection::open(path).map_err(db_error)?;
        db.busy_timeout(std::time::Duration::from_millis(1000))
            .map_err(db_error)?;
        db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA secure_delete=ON;").map_err(db_error)?;
        let version: i64 = db
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(db_error)?;
        let migrations = [include_str!("001_memory.sql"), include_str!("002_fts.sql")];
        if version < 0 || version > migrations.len() as i64 {
            return Err("Unsupported newer database schema".into());
        }
        for (i, sql) in migrations.iter().enumerate().skip(version as usize) {
            let tx = db
                .transaction_with_behavior(TransactionBehavior::Immediate)
                .map_err(db_error)?;
            tx.execute_batch(sql).map_err(db_error)?;
            tx.execute(
                "INSERT INTO schema_migrations VALUES(?1,?2)",
                params![i as i64 + 1, now()],
            )
            .map_err(db_error)?;
            tx.pragma_update(None, "user_version", i as i64 + 1)
                .map_err(db_error)?;
            tx.commit().map_err(db_error)?;
        }
        Ok(Self { db })
    }
    pub fn identity(&self) -> Result<Identity> {
        self.db.query_row("SELECT name,description,personality,instructions,version,created_at,updated_at FROM identity_profile WHERE id=1",[],|r|Ok(Identity{profile:Profile{name:r.get(0)?,description:r.get(1)?,personality:r.get(2)?,instructions:r.get(3)?},version:r.get(4)?,created_at:r.get(5)?,updated_at:r.get(6)?})).optional().map_err(db_error)?.ok_or("Configure Lucille identity explicitly before use".into())
    }
    pub fn configure_identity(
        &mut self,
        profile: Profile,
        expected_version: i64,
        _permit: &ConfigurationPermit,
    ) -> Result<Identity> {
        profile.validate()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let current: i64 = tx
            .query_row(
                "SELECT coalesce((SELECT version FROM identity_profile WHERE id=1),0)",
                [],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if current != expected_version {
            return Err("Identity version changed".into());
        }
        tx.execute("INSERT INTO identity_profile VALUES(1,?1,?2,?3,?4,1,?5,?5) ON CONFLICT(id) DO UPDATE SET name=excluded.name,description=excluded.description,personality=excluded.personality,instructions=excluded.instructions,version=identity_profile.version+1,updated_at=excluded.updated_at",params![profile.name,profile.description,profile.personality,profile.instructions,now()]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        self.identity()
    }
    pub fn list(&self, scope: &str, project: &str) -> Result<Vec<Memory>> {
        identifier(scope)?;
        identifier(project)?;
        let mut q=self.db.prepare(&format!("SELECT {COLS} FROM memories WHERE scope=?1 AND project=?2 ORDER BY created_at,id LIMIT 1000")).map_err(db_error)?;
        let out = q
            .query_map(params![scope, project], row)
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        Ok(out)
    }
    fn get(&self, id: &str, scope: &str, project: &str) -> Result<Memory> {
        uuid::Uuid::parse_str(id).map_err(|_| "Invalid memory ID")?;
        identifier(scope)?;
        identifier(project)?;
        self.db
            .query_row(
                &format!("SELECT {COLS} FROM memories WHERE id=?1 AND scope=?2 AND project=?3"),
                params![id, scope, project],
                row,
            )
            .optional()
            .map_err(db_error)?
            .ok_or("Memory not found in scope".into())
    }
    pub fn create_candidate(&mut self, input: Candidate) -> Result<Memory> {
        input.validate()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let existing = tx
            .query_row(
                &format!("SELECT {COLS} FROM memories WHERE id=?1"),
                [&input.id],
                row,
            )
            .optional()
            .map_err(db_error)?;
        if let Some(old) = existing {
            if old.input != input {
                return Err("Idempotency ID already used".into());
            }
            return Ok(old);
        }
        let deleted: bool = tx
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM history WHERE record_id=?1 AND operation='delete')",
                [&input.id],
                |r| r.get(0),
            )
            .map_err(db_error)?;
        if deleted {
            return Err("Deleted memory ID cannot be reused".into());
        }
        // Repetition with a different request ID is also deduplicated within the same provenance/key.
        let duplicate=tx.query_row(&format!("SELECT {COLS} FROM memories WHERE scope=?1 AND project=?2 AND kind=?3 AND content=?4 AND fact_key=?5 AND source=?6 AND state!='obsolete' LIMIT 1"),params![input.scope,input.project,types::label(&input.kind),input.content,input.fact_key,types::label(&input.source)],row).optional().map_err(db_error)?;
        if let Some(old) = duplicate {
            return Ok(old);
        }
        tx.execute("INSERT OR IGNORE INTO scopes VALUES(?1)", [&input.scope])
            .map_err(db_error)?;
        tx.execute(
            "INSERT INTO memories VALUES(?1,?2,?3,?4,'candidate',?5,?6,?7,?8,1,?9,?9)",
            params![
                input.id,
                input.scope,
                input.project,
                types::label(&input.kind),
                input.content,
                input.fact_key,
                types::label(&input.source),
                input.source_id,
                now()
            ],
        )
        .map_err(db_error)?;
        tx.execute("INSERT INTO history(memory_id,record_id,operation,version,at) VALUES(?1,?1,'create',1,?2)",params![input.id,now()]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        self.get(&input.id, &input.scope, &input.project)
    }
    /// All transitions use optimistic versions. Conflicting keys require explicit obsoletion first.
    pub fn change(
        &mut self,
        id: &str,
        scope: &str,
        project: &str,
        version: i64,
        action: Action,
    ) -> Result<Option<Memory>> {
        let old = self.get(id, scope, project)?;
        if old.version != version {
            return Err("Memory version changed".into());
        }
        if let Action::Update { content, fact_key } = &action {
            text(content, 16000)?;
            identifier(fact_key)?;
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let current: i64 = tx
            .query_row("SELECT version FROM memories WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(db_error)?;
        if current != version {
            return Err("Memory version changed".into());
        }
        let mut state = old.state.clone();
        let mut content = old.input.content.clone();
        let mut key = old.input.fact_key.clone();
        let op = match &action {
            Action::Approve => {
                if !matches!(state, Status::Candidate | Status::Conflict) {
                    return Err("Only candidates or conflicts can be approved".into());
                }
                let n:i64=tx.query_row("SELECT count(*) FROM memories WHERE scope=?1 AND project=?2 AND fact_key=?3 AND state='active' AND id!=?4",params![scope,project,key,id],|r|r.get(0)).map_err(db_error)?;
                state = if n > 0 {
                    Status::Conflict
                } else {
                    Status::Active
                };
                "approve"
            }
            Action::Reject => {
                if !matches!(state, Status::Candidate | Status::Conflict) {
                    return Err("Only candidates can be rejected".into());
                }
                state = Status::Obsolete;
                "reject"
            }
            Action::Obsolete => {
                state = Status::Obsolete;
                "obsolete"
            }
            Action::Update {
                content: c,
                fact_key: k,
            } => {
                content = c.clone();
                key = k.clone();
                state = Status::Candidate;
                "update"
            }
            Action::Delete => "delete",
        };
        if matches!(action, Action::Delete) {
            tx.execute("DELETE FROM memories WHERE id=?1", [id])
                .map_err(db_error)?;
        } else {
            tx.execute("UPDATE memories SET state=?1,content=?2,fact_key=?3,version=version+1,updated_at=?4 WHERE id=?5",params![types::label(&state),content,key,now(),id]).map_err(db_error)?;
        }
        tx.execute(
            "INSERT INTO history(memory_id,record_id,operation,version,at) VALUES(?1,?2,?3,?4,?5)",
            params![
                if matches!(action, Action::Delete) {
                    None
                } else {
                    Some(id)
                },
                id,
                op,
                version + 1,
                now()
            ],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        if matches!(action, Action::Delete) {
            Ok(None)
        } else {
            self.get(id, scope, project).map(Some)
        }
    }
    pub fn history(&self, id: &str) -> Result<Vec<Change>> {
        uuid::Uuid::parse_str(id).map_err(|_| "Invalid memory ID")?;
        let mut q=self.db.prepare("SELECT record_id,operation,version,at FROM history WHERE record_id=?1 ORDER BY sequence").map_err(db_error)?;
        let out = q
            .query_map([id], |r| {
                Ok(Change {
                    record_id: r.get(0)?,
                    operation: r.get(1)?,
                    version: r.get(2)?,
                    at: r.get(3)?,
                })
            })
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        Ok(out)
    }
    /// A fresh read transaction on EVERY send, including agent steps and retries.
    pub fn context(&mut self, req: TurnRequest) -> Result<TurnContext> {
        identifier(&req.conversation_id)?;
        identifier(&req.message_id)?;
        identifier(&req.scope)?;
        identifier(&req.project)?;
        if req.token_budget < 512 || req.token_budget > 16384 || req.query.len() > 32000 {
            return Err("Invalid context budget or query".into());
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let identity=tx.query_row("SELECT name,description,personality,instructions,version,created_at,updated_at FROM identity_profile WHERE id=1",[],|r|Ok(Identity{profile:Profile{name:r.get(0)?,description:r.get(1)?,personality:r.get(2)?,instructions:r.get(3)?},version:r.get(4)?,created_at:r.get(5)?,updated_at:r.get(6)?})).optional().map_err(db_error)?.ok_or("Lucille identity is not configured")?;
        // JSON bytes are a deliberately conservative token ceiling, including framing allowance.
        let mut used = serde_json::to_vec(&identity.profile)
            .map_err(|_| "Invalid profile")?
            .len()
            + 384;
        if used > req.token_budget {
            return Err("Identity exceeds context budget".into());
        }
        let available = req.token_budget - used;
        let query = req
            .query
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .take(24)
            .map(|s| format!("\"{s}\""))
            .collect::<Vec<_>>()
            .join(" OR ");
        let mut picks = Vec::new();
        if !query.is_empty() {
            let mut q=tx.prepare("SELECT m.id,m.scope,m.project,m.kind,m.state,m.content,m.fact_key,m.source,m.source_id,m.version,m.created_at,m.updated_at FROM memories_fts JOIN memories m ON m.rowid=memories_fts.rowid WHERE memories_fts MATCH ?1 AND m.scope=?2 AND m.project=?3 AND m.state='active' ORDER BY bm25(memories_fts),m.id LIMIT 100").map_err(db_error)?;
            picks = q
                .query_map(params![query, req.scope, req.project], row)
                .map_err(db_error)?
                .collect::<rusqlite::Result<Vec<_>>>()
                .map_err(db_error)?;
        }
        // Durable preferences have a reserved share, capped so they cannot crowd out relevant facts.
        let mut prefs=tx.prepare(&format!("SELECT {COLS} FROM memories WHERE scope=?1 AND project=?2 AND state='active' AND kind='preference' ORDER BY updated_at DESC,id LIMIT 100")).map_err(db_error)?;
        let preferences = prefs
            .query_map(params![req.scope, req.project], row)
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        drop(prefs);
        let mut memories = Vec::new();
        let mut pref_used = 0;
        for m in preferences.into_iter().chain(
            picks
                .into_iter()
                .filter(|m| m.input.kind != Kind::Preference),
        ) {
            let cost = serde_json::to_vec(&m).map_err(|_| "Invalid memory")?.len() + 32;
            if m.input.kind == Kind::Preference && pref_used + cost > available / 4 {
                continue;
            }
            if used + cost > req.token_budget
                || memories.iter().any(|x: &Memory| x.input.id == m.input.id)
            {
                continue;
            }
            if m.input.kind == Kind::Preference {
                pref_used += cost;
            }
            used += cost;
            memories.push(m);
        }
        let references: Vec<_> = memories
            .iter()
            .map(|m| Reference {
                id: m.input.id.clone(),
                version: m.version,
            })
            .collect();
        tx.execute("INSERT OR IGNORE INTO scopes VALUES(?1)", [&req.scope])
            .map_err(db_error)?;
        tx.execute("INSERT INTO retrieval_audit(conversation_id,message_id,mode,scope,project,identity_version,references_json,budget,used,at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![req.conversation_id,req.message_id,types::label(&req.mode),req.scope,req.project,identity.version,serde_json::to_string(&references).map_err(|_|"Invalid references")?,req.token_budget as i64,used as i64,now()]).map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(TurnContext {
            conversation_id: req.conversation_id,
            message_id: req.message_id,
            scope: req.scope,
            project: req.project,
            mode: req.mode,
            identity,
            memories,
            references,
            token_budget: req.token_budget,
            used_upper_bound: used,
            retrieval_state: "lexical_fresh_snapshot".into(),
        })
    }
    pub fn check_integrity(&self) -> Result<()> {
        let check: String = self
            .db
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .map_err(db_error)?;
        if check != "ok" {
            return Err("SQLite integrity check failed".into());
        }
        let foreign: bool = self
            .db
            .prepare("PRAGMA foreign_key_check")
            .map_err(db_error)?
            .exists([])
            .map_err(db_error)?;
        if foreign {
            return Err("Foreign key check failed".into());
        }
        self.db
            .execute(
                "INSERT INTO memories_fts(memories_fts,rank) VALUES('integrity-check',1)",
                [],
            )
            .map_err(db_error)?;
        Ok(())
    }
    /// Logical backup, data only: restoration never executes SQL supplied by a backup.
    pub fn export_backup(&self, _permit: &ConfigurationPermit) -> Result<String> {
        let tx = self.db.unchecked_transaction().map_err(db_error)?;
        let mut q = self
            .db
            .prepare(&format!("SELECT {COLS} FROM memories ORDER BY id"))
            .map_err(db_error)?;
        let memories = q
            .query_map([], row)
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        drop(q);
        let mut q = tx
            .prepare("SELECT record_id,operation,version,at FROM history ORDER BY sequence")
            .map_err(db_error)?;
        let history = q
            .query_map([], |r| {
                Ok(Change {
                    record_id: r.get(0)?,
                    operation: r.get(1)?,
                    version: r.get(2)?,
                    at: r.get(3)?,
                })
            })
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        drop(q);
        let mut q = tx.prepare("SELECT conversation_id,message_id,mode,scope,project,identity_version,references_json,budget,used,at FROM retrieval_audit ORDER BY sequence").map_err(db_error)?;
        let audits = q
            .query_map([], |r| {
                let encoded: String = r.get(6)?;
                let references = serde_json::from_str(&encoded).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?;
                Ok(RetrievalAudit {
                    conversation_id: r.get(0)?,
                    message_id: r.get(1)?,
                    mode: types::parse(r.get(2)?)?,
                    scope: r.get(3)?,
                    project: r.get(4)?,
                    identity_version: r.get(5)?,
                    references,
                    budget: r.get(7)?,
                    used: r.get(8)?,
                    at: r.get(9)?,
                })
            })
            .map_err(db_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(db_error)?;
        drop(q);
        let result = serde_json::to_string(&Backup {
            schema: 2,
            identity: self.identity()?,
            memories,
            history,
            audits,
        })
        .map_err(|_| "Backup serialization failed".to_string())?;
        tx.commit().map_err(db_error)?;
        Ok(result)
    }
    /// Isolated test databases only. Host must never expose this on an operational profile.
    pub fn restore_isolated(&mut self, json: &str, _permit: &ConfigurationPermit) -> Result<()> {
        if json.len() > 16 * 1024 * 1024 {
            return Err("Backup too large".into());
        }
        let backup: Backup = serde_json::from_str(json).map_err(|_| "Invalid backup")?;
        if backup.schema != 2 {
            return Err("Unsupported backup schema".into());
        }
        backup.identity.profile.validate()?;
        if backup.identity.version < 1 || backup.memories.len() > 10000 {
            return Err("Invalid backup".into());
        }
        for m in &backup.memories {
            m.input.validate()?;
            if m.version < 1 {
                return Err("Invalid memory version".into());
            }
        }
        let mut active_keys = std::collections::HashSet::new();
        for m in backup.memories.iter().filter(|m| m.state == Status::Active) {
            if !active_keys.insert((&m.input.scope, &m.input.project, &m.input.fact_key)) {
                return Err("Conflicting active keys in backup".into());
            }
        }
        for change in &backup.history {
            uuid::Uuid::parse_str(&change.record_id).map_err(|_| "Invalid history ID")?;
            if change.version < 1
                || ![
                    "create", "approve", "reject", "obsolete", "update", "delete", "restore",
                ]
                .contains(&change.operation.as_str())
            {
                return Err("Invalid history".into());
            }
        }
        for audit in &backup.audits {
            identifier(&audit.conversation_id)?;
            identifier(&audit.message_id)?;
            identifier(&audit.scope)?;
            identifier(&audit.project)?;
            if audit.identity_version < 1
                || audit.budget < 512
                || audit.budget > 16384
                || audit.used < 0
                || audit.used > audit.budget
            {
                return Err("Invalid audit".into());
            }
            for reference in &audit.references {
                uuid::Uuid::parse_str(&reference.id).map_err(|_| "Invalid audit reference")?;
                if reference.version < 1 {
                    return Err("Invalid audit version".into());
                }
            }
        }
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        tx.execute_batch("DELETE FROM retrieval_audit;DELETE FROM history;DELETE FROM memories;DELETE FROM scopes;DELETE FROM identity_profile;").map_err(db_error)?;
        let i = backup.identity;
        tx.execute(
            "INSERT INTO identity_profile VALUES(1,?1,?2,?3,?4,?5,?6,?7)",
            params![
                i.profile.name,
                i.profile.description,
                i.profile.personality,
                i.profile.instructions,
                i.version,
                i.created_at,
                i.updated_at
            ],
        )
        .map_err(db_error)?;
        for m in backup.memories {
            tx.execute("INSERT OR IGNORE INTO scopes VALUES(?1)", [&m.input.scope])
                .map_err(db_error)?;
            tx.execute(
                "INSERT INTO memories VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
                params![
                    m.input.id,
                    m.input.scope,
                    m.input.project,
                    types::label(&m.input.kind),
                    types::label(&m.state),
                    m.input.content,
                    m.input.fact_key,
                    types::label(&m.input.source),
                    m.input.source_id,
                    m.version,
                    m.created_at,
                    m.updated_at
                ],
            )
            .map_err(db_error)?;
        }
        for change in backup.history {
            tx.execute("INSERT INTO history(memory_id,record_id,operation,version,at) VALUES((SELECT id FROM memories WHERE id=?1),?1,?2,?3,?4)",params![change.record_id,change.operation,change.version,change.at]).map_err(db_error)?;
        }
        tx.execute("INSERT INTO history(memory_id,record_id,operation,version,at) SELECT id,id,'restore',version,?1 FROM memories",[now()]).map_err(db_error)?;
        for audit in backup.audits {
            tx.execute("INSERT OR IGNORE INTO scopes VALUES(?1)", [&audit.scope])
                .map_err(db_error)?;
            tx.execute("INSERT INTO retrieval_audit(conversation_id,message_id,mode,scope,project,identity_version,references_json,budget,used,at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",params![audit.conversation_id,audit.message_id,types::label(&audit.mode),audit.scope,audit.project,audit.identity_version,serde_json::to_string(&audit.references).map_err(|_|"Invalid references")?,audit.budget,audit.used,audit.at]).map_err(db_error)?;
        }
        tx.execute(
            "INSERT INTO memories_fts(memories_fts,rank) VALUES('integrity-check',1)",
            [],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)?;
        Ok(())
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum Action {
    Approve,
    Reject,
    Obsolete,
    Delete,
    Update { content: String, fact_key: String },
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Backup {
    schema: u32,
    identity: Identity,
    memories: Vec<Memory>,
    history: Vec<Change>,
    audits: Vec<RetrievalAudit>,
}
