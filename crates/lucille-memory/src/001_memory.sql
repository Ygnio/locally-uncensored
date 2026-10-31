CREATE TABLE schema_migrations(version INTEGER PRIMARY KEY, applied_at INTEGER NOT NULL);
CREATE TABLE identity_profile(
 id INTEGER PRIMARY KEY CHECK(id=1), name TEXT NOT NULL, description TEXT NOT NULL,
 personality TEXT NOT NULL, instructions TEXT NOT NULL, version INTEGER NOT NULL,
 created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE TABLE scopes(id TEXT PRIMARY KEY);
CREATE TABLE memories(
 id TEXT PRIMARY KEY, scope TEXT NOT NULL REFERENCES scopes(id), project TEXT NOT NULL,
 kind TEXT NOT NULL CHECK(kind IN ('fact','preference','hardware','project','decision','solution','goal','task','constraint','summary','hypothesis')),
 state TEXT NOT NULL CHECK(state IN ('candidate','active','conflict','obsolete')),
 content TEXT NOT NULL, fact_key TEXT NOT NULL, source TEXT NOT NULL, source_id TEXT NOT NULL,
 version INTEGER NOT NULL CHECK(version>0), created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL
);
CREATE INDEX scoped_memories ON memories(scope,project,state,kind);
CREATE TABLE history(
 sequence INTEGER PRIMARY KEY, memory_id TEXT REFERENCES memories(id) ON DELETE SET NULL,
 record_id TEXT NOT NULL, operation TEXT NOT NULL, version INTEGER NOT NULL, at INTEGER NOT NULL
);
CREATE TABLE retrieval_audit(
 sequence INTEGER PRIMARY KEY, conversation_id TEXT NOT NULL, message_id TEXT NOT NULL,
 mode TEXT NOT NULL CHECK(mode IN ('chat','agent')), scope TEXT NOT NULL REFERENCES scopes(id),
 project TEXT NOT NULL, identity_version INTEGER NOT NULL, references_json TEXT NOT NULL,
 budget INTEGER NOT NULL, used INTEGER NOT NULL, at INTEGER NOT NULL
);
