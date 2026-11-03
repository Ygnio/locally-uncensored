CREATE TABLE learning_inputs(
 job_id TEXT PRIMARY KEY REFERENCES extraction_jobs(id) ON DELETE CASCADE,
 user_text TEXT NOT NULL CHECK(length(CAST(user_text AS BLOB))<=1600),
 extraction_mode TEXT NOT NULL CHECK(extraction_mode IN ('llm','hybrid'))
);
CREATE TRIGGER purge_learning_input AFTER UPDATE OF state ON extraction_jobs
WHEN NEW.state IN ('done','cancelled','failed') BEGIN
 DELETE FROM learning_inputs WHERE job_id=NEW.id;
END;
