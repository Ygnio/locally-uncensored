//! Synthetic data with real model outputs; canonical Store is the only writer.
use lucille_memory::{intelligent::*, llm, semantic::*, *};
use serde_json::{json, Value};
use std::{io::Read, path::PathBuf};
fn run() -> Result<Value> {
    let supplied = PathBuf::from(std::env::var_os("LUCILLE_QA_ROOT").ok_or("QA root required")?);
    if !supplied.is_absolute() {
        return Err("Absolute QA root required".into());
    }
    let root = supplied
        .canonicalize()
        .map_err(|_| "Existing QA root required")?;
    let checkout = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(".lucille-qa-runtime")
        .canonicalize()
        .map_err(|_| "Checkout QA root required")?;
    if !root.starts_with(&checkout) {
        return Err("Bridge may only write checkout-owned QA data".into());
    }
    if !root.is_absolute()
        || !root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .starts_with("lucille-synthetic-")
        || std::fs::read(root.join(".lucille-isolated")).map_err(|_| "Missing marker")?
            != b"synthetic-data-only\n"
    {
        return Err("Marked synthetic root required".into());
    }
    let mut input = String::new();
    std::io::stdin()
        .take(1024 * 1024)
        .read_to_string(&mut input)
        .map_err(|_| "Input failed")?;
    let v: Value = serde_json::from_str(&input).map_err(|_| "Invalid command")?;
    for name in [
        "synthetic.sqlite3",
        "synthetic.sqlite3-wal",
        "synthetic.sqlite3-shm",
    ] {
        let path = root.join(name);
        if let Ok(meta) = path.symlink_metadata() {
            if meta.file_type().is_symlink()
                || path.canonicalize().map_err(|_| "Invalid DB path")?.parent()
                    != Some(root.as_path())
            {
                return Err("Linked database path forbidden".into());
            }
        }
    }
    let database = root.join("synthetic.sqlite3");
    let fresh = !database.exists();
    let mut s = Store::open(&database)?;
    let permit = ConfigurationPermit::explicit_settings_action(true)?;
    let scope = v["scope"].as_str().unwrap_or("lucille");
    let project = v["project"].as_str().unwrap_or("atlas");
    let at = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "Clock failed")?
        .as_secs() as i64;
    match v["operation"].as_str().ok_or("Missing operation")? {
        "configure_identity" => {
            let profile: Profile = serde_json::from_value(v["profile"].clone())
                .map_err(|_| "Invalid synthetic profile")?;
            let expected = v["expected_version"].as_i64().ok_or("Missing identity version")?;
            Ok(json!(s.configure_identity(profile, expected, &permit)?))
        }
        "learn" => {
            if s.identity().is_err() {
                s.configure_identity(Profile::fixture(), 0, &permit)?;
            }
            let mode: llm::ExtractionMode =
                serde_json::from_value(v["mode"].clone()).map_err(|_| "Invalid extraction mode")?;
            let mut policy: LearningPolicy = match v.get("learning_policy") {
                Some(raw) => serde_json::from_value(raw.clone()).map_err(|_| "Invalid synthetic learning policy")?,
                None => LearningPolicy::default(),
            };
            policy.enabled = true;
            policy.autonomous_preferences = v["autonomy"].as_bool().unwrap_or(false);
            policy.automatic_indexing = v["automatic_indexing"].as_bool().unwrap_or(false);
            policy.extraction_mode = mode;
            s.set_learning_policy(policy, &permit)?;
            let text = v["text"].as_str().ok_or("Missing text")?;
            let enqueue = s.enqueue_learning(Interaction {
                conversation_id: v["conversation_id"]
                    .as_str()
                    .unwrap_or("real-model-synthetic")
                    .into(),
                message_id: v["message_id"].as_str().unwrap_or("case").into(),
                scope: scope.into(),
                project: project.into(),
                mode: serde_json::from_value(v.get("interaction_mode").cloned().unwrap_or(json!("chat")))
                    .map_err(|_| "Invalid interaction mode")?,
                role: "user".into(),
                user_text: text.into(),
            })?;
            let mut rejected = None;
            let output = v["output"].as_str().unwrap_or("{\"proposals\":[]}");
            let validation = llm::validate(text, scope, project, output);
            if let Some(lease) = s.claim_learning(at + 1)? {
                s.complete_learning(&lease, &lease.claims, at + 2)?;
            } else if let Some(lease) = s.claim_llm_learning(at + 1)? {
                if let Err(error) = s.complete_llm_learning(&lease, output, at + 2) {
                    rejected = Some(error);
                    // This one-shot diagnostic has no retry worker. Native validation
                    // already revoked the failed lease; terminate the job observably.
                    // Valid hybrid empty-output fallback is handled inside Store.
                    s.cancel_learning(&lease.id, &permit)?;
                }
            }
            Ok(
                json!({"enqueue":enqueue,"validation": match validation {Ok(c)=>json!({"accepted":c}),Err(e)=>json!({"error":e})},"rejected":rejected,"details":s.memory_details(scope,project)?,"jobs":s.learning_jobs(scope,project)?,"identity":s.identity()?}),
            )
        }
        "learning_config" => {
            let p: LearningPolicy =
                serde_json::from_value(v["policy"].clone()).map_err(|_| "Invalid policy")?;
            s.set_learning_policy(p, &permit)?;
            Ok(json!(s.learning_policy()?))
        }
        "metrics" => s.learning_metrics(scope, project),
        "answer_evidence" => {
            let req: TurnRequest =
                serde_json::from_value(v["request"].clone()).map_err(|_| "Invalid request")?;
            let refs: Vec<Reference> = serde_json::from_value(v["references"].clone())
                .map_err(|_| "Invalid references")?;
            let claims: Vec<lucille_memory::context_facts::ClaimedField> =
                serde_json::from_value(v.get("claims").cloned().unwrap_or(json!([])))
                    .map_err(|_| "Invalid claimed fields")?;
            Ok(json!(s.answer_evidence_for_claims(&req, &refs, &claims)?))
        }
        "pending_embeddings" => {
            s.pending_embedding_records(scope, project, v["limit"].as_u64().unwrap_or(8) as usize)
        }
        "export" => Ok(json!(s.export_backup(&permit)?)),
        "restore_canonical_fixture" => {
            let phase_root = checkout.join("lucille-synthetic-real22f");
            let relative = root.strip_prefix(&phase_root).map_err(|_| "C1 root required")?;
            if !fresh
                || relative.components().count() != 2
                || !root.file_name().unwrap_or_default().to_string_lossy().starts_with("lucille-synthetic-c1-")
                || std::fs::read(phase_root.join(".lucille-isolated")).map_err(|_| "Missing phase marker")?
                    != b"synthetic-data-only\n"
            {
                return Err("Fresh marked C1 fixture store required".into());
            }
            let backup = v["backup"].as_str().ok_or("Missing synthetic canonical fixture")?;
            s.restore_isolated(backup, &permit)?;
            s.check_integrity()?;
            Ok(json!(s.identity()?))
        }
        "details" => Ok(json!(s.memory_details(scope, project)?)),
        "deleted" => Ok(json!(s.deleted_memories(scope, project)?)),
        "representation" => s.memory_embedding_text(
            v["id"].as_str().ok_or("Missing id")?,
            scope,
            project,
            v["style"].as_str().unwrap_or("natural"),
        ),
        "normalize" => Ok(json!(lucille_memory::normalization::normalize(
            v["text"].as_str().ok_or("Missing text")?,
            scope,
            project,
            v["output"].as_str().ok_or("Missing output")?
        )?)),
        "explain" => {
            let request: TurnRequest =
                serde_json::from_value(v["request"].clone()).map_err(|_| "Invalid request")?;
            let vector: Option<QueryVector> =
                serde_json::from_value(v.get("query_vector").cloned().unwrap_or(Value::Null))
                    .map_err(|_| "Invalid vector")?;
            let offset = v["logical_offset_seconds"].as_i64().unwrap_or(0);
            if !(0..=604800).contains(&offset) {
                return Err("QA logical offset ceiling".into());
            }
            s.explain_context_at(request, vector.as_ref(), at + offset)
        }
        "create" => {
            let candidate: Candidate =
                serde_json::from_value(v["candidate"].clone()).map_err(|_| "Invalid candidate")?;
            Ok(json!(s.create_candidate(candidate)?))
        }
        "change" => {
            let action: Action =
                serde_json::from_value(v["action"].clone()).map_err(|_| "Invalid action")?;
            Ok(json!(s.change(
                v["id"].as_str().ok_or("Missing id")?,
                scope,
                project,
                v["version"].as_i64().ok_or("Missing version")?,
                action
            )?))
        }
        "policy" => {
            let mut policy = s.learning_policy()?;
            policy.lexical_weight = v["lexical_weight"].as_f64().ok_or("Missing weight")?;
            policy.semantic_weight = 1. - policy.lexical_weight;
            if let Some(minimum) = v["minimum_similarity"].as_f64() {
                policy.minimum_similarity = minimum;
            }
            s.set_learning_policy(policy.clone(), &permit)?;
            Ok(json!(policy))
        }
        "configure_embedding" => {
            let space: EmbeddingSpace =
                serde_json::from_value(v["space"].clone()).map_err(|_| "Invalid space")?;
            s.configure_embedding_space(&space, &permit)?;
            Ok(s.embedding_status()?)
        }
        "put_vector" => {
            let space: EmbeddingSpace =
                serde_json::from_value(v["space"].clone()).map_err(|_| "Invalid space")?;
            let vector: Vec<f32> =
                serde_json::from_value(v["vector"].clone()).map_err(|_| "Invalid vector")?;
            Ok(json!(s.put_memory_vector(
                v["id"].as_str().ok_or("Missing id")?,
                v["version"].as_i64().ok_or("Missing version")?,
                &space,
                &vector,
                &permit
            )?))
        }
        "context" => {
            let req: TurnRequest =
                serde_json::from_value(v["request"].clone()).map_err(|_| "Invalid request")?;
            let vector: Option<QueryVector> =
                serde_json::from_value(v.get("query_vector").cloned().unwrap_or(Value::Null))
                    .map_err(|_| "Invalid vector")?;
            Ok(json!(s.context_hybrid(req, vector.as_ref())?))
        }
        _ => Err("Unsupported synthetic operation".into()),
    }
}
fn main() {
    match run() {
        Ok(v) => println!("{v}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1)
        }
    }
}
