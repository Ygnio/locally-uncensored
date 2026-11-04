//! Versioned vector mechanics, with no implicit endpoint or model loading.
use crate::{db_error, extraction::digest, ConfigurationPermit, Result, Store};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};

pub(crate) fn prune_in_transaction(tx: &rusqlite::Transaction) -> Result<usize> {
    tx.execute("DELETE FROM memory_vectors WHERE NOT EXISTS(SELECT 1 FROM memories m JOIN embedding_spaces s ON s.id=memory_vectors.space_id WHERE m.id=memory_vectors.memory_id AND m.state='active' AND m.version=memory_vectors.memory_version AND s.active=1)", []).map_err(db_error)
}
impl Store {
    /// Derived data only. Canonical revisions, sources and tombstones remain untouched.
    pub fn prune_obsolete_vectors(&mut self) -> Result<usize> {
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let removed = prune_in_transaction(&tx)?;
        tx.commit().map_err(db_error)?;
        Ok(removed)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EmbeddingSpace {
    pub model: String,
    pub revision: String,
    pub fingerprint: String,
    pub dimension: usize,
    pub normalization: String,
    pub pooling: String,
    pub document_prefix: String,
    pub query_prefix: String,
    pub context_tokens: usize,
    pub index_version: u32,
}
impl EmbeddingSpace {
    pub fn validate(&self) -> Result<()> {
        if [
            &self.model,
            &self.revision,
            &self.fingerprint,
            &self.normalization,
            &self.pooling,
            &self.document_prefix,
            &self.query_prefix,
        ]
        .iter()
        .any(|s| s.is_empty() || s.len() > 512)
            || self.fingerprint.len() != 64
            || !self.fingerprint.chars().all(|c| c.is_ascii_hexdigit())
            || self.dimension == 0
            || self.dimension > 4096
            || self.context_tokens == 0
            || self.context_tokens > 8192
            || self.index_version == 0
            || !matches!(self.normalization.as_str(), "l2" | "layer_norm_then_l2")
        {
            return Err("Invalid embedding manifest".into());
        }
        if self.model == "nomic-ai/nomic-embed-text-v1.5"
            && (![64, 128, 256, 512, 768].contains(&self.dimension)
                || self.document_prefix != "search_document: "
                || self.query_prefix != "search_query: "
                || self.pooling != "mean")
        {
            return Err("Nomic manifest mismatch".into());
        }
        Ok(())
    }
    pub fn id(&self) -> Result<String> {
        self.validate()?;
        Ok(digest(
            &serde_json::to_string(self).map_err(|_| "Invalid embedding manifest")?,
        ))
    }
    pub fn check_vector(&self, vector: &[f32]) -> Result<()> {
        if vector.len() != self.dimension || vector.iter().any(|v| !v.is_finite()) {
            return Err("Invalid embedding vector".into());
        }
        let norm = vector
            .iter()
            .map(|v| (*v as f64).powi(2))
            .sum::<f64>()
            .sqrt();
        if (norm - 1.0).abs() > 0.001 {
            return Err("Expected explicitly normalized vector".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryVector {
    pub space: EmbeddingSpace,
    pub vector: Vec<f32>,
}
/// A future isolated client must verify effective identity/auth/context/resource
/// availability before implementing this interface. No implementation is instantiated here.
pub trait EmbeddingClient {
    fn manifest(&self) -> Result<EmbeddingSpace>;
    fn encode(&self, text: &str) -> Result<Vec<f32>>;
}
pub fn encode_query(
    client: &dyn EmbeddingClient,
    expected: &EmbeddingSpace,
    text: &str,
) -> Result<QueryVector> {
    expected.validate()?;
    if client.manifest()? != *expected {
        return Err("Effective embedding model changed".into());
    }
    let input = format!("{}{text}", expected.query_prefix);
    // UTF-8 byte count is a deliberately conservative token upper bound.
    if input.len() > expected.context_tokens {
        return Err("Embedding input exceeds context ceiling".into());
    }
    let vector = client.encode(&input)?;
    expected.check_vector(&vector)?;
    if client.manifest()? != *expected {
        return Err("Embedding fingerprint changed during request".into());
    }
    Ok(QueryVector {
        space: expected.clone(),
        vector,
    })
}
impl Store {
    pub fn embedding_status(&self) -> Result<serde_json::Value> {
        let manifest: Option<String> = self
            .db
            .query_row(
                "SELECT spec_json FROM embedding_spaces WHERE active=1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_error)?;
        let indexed:i64=self.db.query_row("SELECT count(*) FROM memory_vectors v JOIN memories m ON m.id=v.memory_id WHERE m.version=v.memory_version AND m.state='active'",[],|r|r.get(0)).map_err(db_error)?;
        Ok(
            serde_json::json!({"space":manifest.map(|s|serde_json::from_str::<EmbeddingSpace>(&s).map_err(|_|"Invalid manifest")).transpose()?,"indexed":indexed,"transport_verified":false}),
        )
    }

    pub fn configure_embedding_space(
        &mut self,
        spec: &EmbeddingSpace,
        _permit: &ConfigurationPermit,
    ) -> Result<String> {
        let id = spec.id()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let current: Option<String> = tx
            .query_row("SELECT id FROM embedding_spaces WHERE active=1", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(db_error)?;
        if current.as_ref() != Some(&id) {
            tx.execute("DELETE FROM memory_vectors", [])
                .map_err(db_error)?;
            tx.execute("UPDATE embedding_spaces SET active=0", [])
                .map_err(db_error)?;
            tx.execute("INSERT INTO embedding_spaces VALUES(?1,?2,1) ON CONFLICT(id) DO UPDATE SET active=1",params![id,serde_json::to_string(spec).map_err(|_|"Invalid embedding spec")?]).map_err(db_error)?;
        }
        tx.commit().map_err(db_error)?;
        Ok(id)
    }
    pub fn put_memory_vector(
        &mut self,
        id: &str,
        expected_version: i64,
        spec: &EmbeddingSpace,
        vector: &[f32],
        _permit: &ConfigurationPermit,
    ) -> Result<()> {
        spec.check_vector(vector)?;
        let space = spec.id()?;
        let tx = self
            .db
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let valid:bool=tx.query_row("SELECT EXISTS(SELECT 1 FROM memories m,embedding_spaces s WHERE m.id=?1 AND m.version=?2 AND m.state='active' AND s.id=?3 AND s.active=1)",params![id,expected_version,space],|r|r.get(0)).map_err(db_error)?;
        if !valid {
            return Err("Stale memory version or embedding space".into());
        }
        tx.execute(
            "INSERT OR REPLACE INTO memory_vectors VALUES(?1,?2,?3,?4)",
            params![
                id,
                expected_version,
                space,
                serde_json::to_string(vector).map_err(|_| "Invalid vector")?
            ],
        )
        .map_err(db_error)?;
        tx.commit().map_err(db_error)
    }
    pub fn embed_memory(
        &mut self,
        id: &str,
        scope: &str,
        project: &str,
        client: &dyn EmbeddingClient,
        spec: &EmbeddingSpace,
        permit: &ConfigurationPermit,
    ) -> Result<()> {
        if client.manifest()? != *spec {
            return Err("Effective embedding model mismatch".into());
        }
        let memory = self.get(id, scope, project)?;
        let text = if spec.index_version >= 2 {
            crate::semantic_text::render(
                &memory,
                crate::intelligent::quality(&self.db, id)?.as_ref(),
                if spec.index_version >= 3 {
                    "gloss"
                } else {
                    "natural"
                },
            )?
        } else {
            memory.input.content.clone()
        };
        let input = format!("{}{}", spec.document_prefix, text);
        if input.len() > spec.context_tokens {
            return Err("Embedding document exceeds context ceiling".into());
        }
        let vector = client.encode(&input)?;
        if client.manifest()? != *spec {
            return Err("Embedding fingerprint changed during request".into());
        }
        self.put_memory_vector(id, memory.version, spec, &vector, permit)
    }
}
