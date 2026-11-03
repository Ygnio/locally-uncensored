CREATE TABLE learning_policy(id INTEGER PRIMARY KEY CHECK(id=1), json TEXT NOT NULL);
CREATE TABLE extraction_jobs(
 id TEXT PRIMARY KEY, conversation_id TEXT NOT NULL, message_id TEXT NOT NULL,
 scope TEXT NOT NULL, project TEXT NOT NULL, mode TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('queued','running','retry','done','cancelled','failed')),
 attempts INTEGER NOT NULL DEFAULT 0 CHECK(attempts BETWEEN 0 AND 3),
 evidence_json TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL,
 next_at INTEGER NOT NULL, lease_until INTEGER, lease_token TEXT, error_code TEXT,
 UNIQUE(conversation_id,message_id,scope,project,id)
);
CREATE INDEX extraction_ready ON extraction_jobs(state,next_at,lease_until);
CREATE TABLE memory_quality(
 memory_id TEXT PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE,
 subject TEXT NOT NULL, property TEXT NOT NULL, value TEXT NOT NULL,
 confidence REAL NOT NULL CHECK(confidence BETWEEN 0 AND 1),
 relevance REAL NOT NULL CHECK(relevance BETWEEN 0 AND 1),
 temporal TEXT NOT NULL, expires_at INTEGER, priority INTEGER NOT NULL DEFAULT 0,
 replaces_id TEXT, replaces_version INTEGER, facets_json TEXT NOT NULL,
 evidence_rule TEXT NOT NULL
);
CREATE INDEX memory_claim_topic ON memory_quality(subject,property,value);
CREATE TABLE memory_provenance(
 memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
 interaction_id TEXT NOT NULL, evidence TEXT NOT NULL CHECK(length(evidence)<=300),
 at INTEGER NOT NULL, PRIMARY KEY(memory_id,interaction_id)
);
CREATE TABLE memory_revisions(
 memory_id TEXT NOT NULL REFERENCES memories(id) ON DELETE CASCADE,
 version INTEGER NOT NULL, snapshot TEXT NOT NULL, at INTEGER NOT NULL,
 PRIMARY KEY(memory_id,version)
);
CREATE TABLE memory_tombstones(
 id TEXT PRIMARY KEY, scope TEXT NOT NULL, project TEXT NOT NULL, kind TEXT NOT NULL,
 claim_digest TEXT, deleted_at INTEGER NOT NULL
);
CREATE TABLE embedding_spaces(id TEXT PRIMARY KEY, spec_json TEXT NOT NULL, active INTEGER NOT NULL CHECK(active IN (0,1)));
CREATE UNIQUE INDEX one_embedding_space ON embedding_spaces(active) WHERE active=1;
CREATE TABLE memory_vectors(
 memory_id TEXT PRIMARY KEY REFERENCES memories(id) ON DELETE CASCADE,
 memory_version INTEGER NOT NULL, space_id TEXT NOT NULL REFERENCES embedding_spaces(id),
 vector_json TEXT NOT NULL
);
