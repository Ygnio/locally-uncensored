use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Fact,
    Preference,
    Hardware,
    Project,
    Decision,
    Solution,
    Goal,
    Task,
    Constraint,
    Summary,
    Hypothesis,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Candidate,
    Active,
    Conflict,
    Obsolete,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    User,
    Manual,
    ReviewedImport,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub name: String,
    pub description: String,
    pub personality: String,
    pub instructions: String,
}
impl Profile {
    pub fn fixture() -> Self {
        Self {
            name: "Lucille Test Fixture".into(),
            description: "Fictional QA profile".into(),
            personality: "Clear and concise".into(),
            instructions: "Use synthetic examples.".into(),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.len() > 80
            || self.name.contains(['\n', '\r'])
            || self.description.len() > 2000
            || self.personality.len() > 4000
            || self.instructions.len() > 12000
        {
            return Err("Invalid identity profile".into());
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    pub profile: Profile,
    pub version: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Candidate {
    pub id: String,
    pub scope: String,
    pub project: String,
    pub kind: Kind,
    pub content: String,
    pub fact_key: String,
    pub source: Source,
    pub source_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Memory {
    pub input: Candidate,
    pub state: Status,
    pub version: i64,
    pub created_at: i64,
    pub updated_at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    pub record_id: String,
    pub operation: String,
    pub version: i64,
    pub at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RetrievalAudit {
    pub conversation_id: String,
    pub message_id: String,
    pub mode: Mode,
    pub scope: String,
    pub project: String,
    pub identity_version: i64,
    pub references: Vec<Reference>,
    pub budget: i64,
    pub used: i64,
    pub at: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnRequest {
    pub conversation_id: String,
    pub message_id: String,
    pub scope: String,
    pub project: String,
    pub mode: Mode,
    pub query: String,
    pub token_budget: usize,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Chat,
    Agent,
    Code,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reference {
    pub id: String,
    pub version: i64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TurnContext {
    pub conversation_id: String,
    pub message_id: String,
    pub scope: String,
    pub project: String,
    pub mode: Mode,
    pub identity: Identity,
    pub memories: Vec<Memory>,
    #[serde(default)]
    pub facts: Vec<crate::context_facts::ContextFact>,
    #[serde(default)]
    pub current_statements: Vec<crate::context_facts::ContextFact>,
    pub references: Vec<Reference>,
    pub token_budget: usize,
    pub used_upper_bound: usize,
    pub retrieval_state: String,
    #[serde(default,skip_serializing_if="Vec::is_empty")]
    pub relations: Vec<crate::retrieval::RelationContext>,
}
/// Contract only. No provider is instantiated or contacted in phase 1.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SemanticIndexSpec {
    pub provider: String,
    pub effective_model: String,
    pub dimension: usize,
    pub pooling: String,
    pub normalization: String,
    pub document_prefix: String,
    pub query_prefix: String,
    pub index_version: u32,
}
pub trait CandidateExtractor {
    fn extract(&self, user_text: &str) -> Result<Vec<Candidate>, String>;
}

pub(crate) fn label<T: Serialize>(v: &T) -> String {
    serde_json::to_value(v).unwrap().as_str().unwrap().into()
}
pub(crate) fn parse<T: serde::de::DeserializeOwned>(s: String) -> rusqlite::Result<T> {
    serde_json::from_value(serde_json::Value::String(s)).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}
