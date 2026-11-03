ALTER TABLE retrieval_audit RENAME TO retrieval_audit_phase1;
CREATE TABLE retrieval_audit(
 sequence INTEGER PRIMARY KEY, conversation_id TEXT NOT NULL, message_id TEXT NOT NULL,
 mode TEXT NOT NULL CHECK(mode IN ('chat','agent','code')), scope TEXT NOT NULL REFERENCES scopes(id),
 project TEXT NOT NULL, identity_version INTEGER NOT NULL, references_json TEXT NOT NULL,
 budget INTEGER NOT NULL, used INTEGER NOT NULL, at INTEGER NOT NULL
);
INSERT INTO retrieval_audit SELECT * FROM retrieval_audit_phase1;
DROP TABLE retrieval_audit_phase1;
