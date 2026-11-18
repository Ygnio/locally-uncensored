-- Durable receipt metadata only. No source text or removed record contents.
CREATE TABLE memory_source_operations (
 source_id TEXT PRIMARY KEY REFERENCES extraction_jobs(id) ON DELETE CASCADE,
 identity_version INTEGER NOT NULL CHECK(identity_version > 0),
 action TEXT NOT NULL CHECK(action IN ('learn','forget')),
 property TEXT,
 CHECK((action='learn' AND property IS NULL) OR (action='forget' AND property IS NOT NULL))
);
CREATE TABLE memory_source_operation_records (
 source_id TEXT NOT NULL REFERENCES memory_source_operations(source_id) ON DELETE CASCADE,
 record_id TEXT NOT NULL,
 version INTEGER NOT NULL CHECK(version > 0),
 state TEXT NOT NULL CHECK(state IN ('candidate','active','deleted')),
 effect TEXT NOT NULL CHECK(effect IN ('create','update','delete')),
 PRIMARY KEY(source_id, record_id, version)
);
