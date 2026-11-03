use lucille_memory::{intelligent::*, semantic::*, *};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
fn at() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
fn permit() -> ConfigurationPermit {
    ConfigurationPermit::explicit_settings_action(true).unwrap()
}
fn setup() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut s = Store::open(&dir.path().join("synthetic.sqlite3")).unwrap();
    s.configure_identity(Profile::fixture(), 0, &permit())
        .unwrap();
    s.set_learning_policy(
        LearningPolicy {
            enabled: true,
            ..Default::default()
        },
        &permit(),
    )
    .unwrap();
    (dir, s)
}
fn interaction(text: &str) -> Interaction {
    Interaction {
        conversation_id: "synthetic-conversation".into(),
        message_id: uuid::Uuid::new_v4().to_string(),
        scope: "lucille".into(),
        project: "atlas".into(),
        mode: Mode::Chat,
        role: "user".into(),
        user_text: text.into(),
    }
}
fn learn(s: &mut Store, text: &str) -> Option<String> {
    let result = s.enqueue_learning(interaction(text)).unwrap();
    let id = result["id"].as_str()?.to_string();
    let lease = s.claim_learning(at() + 1).unwrap().unwrap();
    assert!(s
        .complete_learning(&lease, &lease.claims, at() + 2)
        .unwrap());
    Some(id)
}
fn req(query: &str) -> TurnRequest {
    TurnRequest {
        conversation_id: "synthetic-conversation".into(),
        message_id: uuid::Uuid::new_v4().to_string(),
        scope: "lucille".into(),
        project: "atlas".into(),
        mode: Mode::Chat,
        query: query.into(),
        token_budget: 8192,
    }
}
fn approve(s: &mut Store, m: &Memory) -> Memory {
    s.change(
        &m.input.id,
        &m.input.scope,
        &m.input.project,
        m.version,
        Action::Approve,
    )
    .unwrap()
    .unwrap()
}
#[test]
fn frozen_extraction_benchmark_measures_precision_recall_unsafe_writes_and_latency() {
    let fixtures: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/phase2-learning.json")).unwrap();
    let (mut tp, mut fp, mut missed, mut unsafe_writes) = (0, 0, 0, 0);
    let mut failures = Vec::new();
    let (mut enqueue_times, mut process_times) = (Vec::new(), Vec::new());
    for case in &fixtures {
        let (_dir, mut s) = setup();
        let mut input = interaction(case["text"].as_str().unwrap());
        input.role = case["role"].as_str().unwrap_or("user").into();
        let started = Instant::now();
        s.enqueue_learning(input).unwrap();
        enqueue_times.push(started.elapsed().as_secs_f64() * 1000.0);
        let started = Instant::now();
        if let Some(lease) = s.claim_learning(at() + 1).unwrap() {
            s.complete_learning(&lease, &lease.claims, at() + 2)
                .unwrap();
        }
        process_times.push(started.elapsed().as_secs_f64() * 1000.0);
        let records = s.list("lucille", "atlas").unwrap();
        let actual = records
            .first()
            .map(|m| serde_json::to_value(&m.input.kind).unwrap());
        let expected = case["kind"].as_str();
        if let Some(expected) = expected {
            if actual.as_ref().and_then(|v| v.as_str()) == Some(expected) && records.len() == 1 {
                tp += 1
            } else {
                missed += 1;
                fp += records.len();
                failures
                    .push(serde_json::json!({"id":case["id"],"expected":expected,"actual":actual}));
            }
        } else if !records.is_empty() {
            fp += records.len();
            unsafe_writes += records.len();
            failures.push(serde_json::json!({"id":case["id"],"expected":null,"actual":actual}));
        }
        let active = records.iter().filter(|m| m.state == Status::Active).count();
        if (active == 1) != case["auto"].as_bool().unwrap() {
            failures.push(serde_json::json!({"id":case["id"],"auto_promotion_mismatch":true}));
        }
    }
    enqueue_times.sort_by(f64::total_cmp);
    process_times.sort_by(f64::total_cmp);
    let precision = tp as f64 / (tp + fp).max(1) as f64;
    let recall = tp as f64 / (tp + missed).max(1) as f64;
    let report = serde_json::json!({"cases":fixtures.len(),"true_positive":tp,"false_positive":fp,"missed":missed,"extraction_precision":precision,"supported_recall":recall,"unsafe_writes":unsafe_writes,"enqueue_p95_ms":enqueue_times[fixtures.len()*95/100],"process_p95_ms":process_times[fixtures.len()*95/100],"real_generation_unnecessary_memory_reference_rate":null,"failures":failures});
    if let Some(path) = std::env::var_os("LUCILLE_PHASE2_BENCHMARK_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
    assert!(
        precision >= 0.95 && recall >= 0.8 && unsafe_writes == 0 && failures.is_empty(),
        "{report}"
    );
    assert!(
        report["enqueue_p95_ms"].as_f64().unwrap() <= 50.0
            && report["process_p95_ms"].as_f64().unwrap() <= 100.0,
        "{report}"
    );
}
#[test]
fn queue_is_durable_idempotent_and_fenced_after_restart_cancel_and_retry() {
    let (dir, mut s) = setup();
    let input = interaction("Prefiro café sem açúcar.");
    let id = s.enqueue_learning(input.clone()).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();
    let first = s.claim_learning(at() + 1).unwrap().unwrap();
    drop(s);
    let mut s = Store::open(&dir.path().join("synthetic.sqlite3")).unwrap();
    assert!(s.claim_learning(at() + 2).unwrap().is_none());
    let second = s.claim_learning(at() + 32).unwrap().unwrap();
    assert!(!s
        .complete_learning(&first, &first.claims, at() + 33)
        .unwrap());
    assert!(s
        .complete_learning(&second, &second.claims, at() + 33)
        .unwrap());
    assert_eq!(s.enqueue_learning(input).unwrap()["id"], id);
    assert!(s.claim_learning(at() + 34).unwrap().is_none());
    assert_eq!(s.list("lucille", "atlas").unwrap().len(), 1);
    let queued = s
        .enqueue_learning(interaction("Meu PC tem 16 GB de RAM."))
        .unwrap();
    let lease = s.claim_learning(at() + 35).unwrap().unwrap();
    assert!(s
        .cancel_learning(queued["id"].as_str().unwrap(), &permit())
        .unwrap());
    assert!(!s
        .complete_learning(&lease, &lease.claims, at() + 36)
        .unwrap());
    let failed = s
        .enqueue_learning(interaction("Meu objetivo é aprender Rust."))
        .unwrap();
    let lease = s.claim_learning(at() + 37).unwrap().unwrap();
    s.fail_learning(&lease, at() + 38).unwrap();
    let lease = s.claim_learning(at() + 50).unwrap().unwrap();
    s.fail_learning(&lease, at() + 51).unwrap();
    let lease = s.claim_learning(at() + 65).unwrap().unwrap();
    s.fail_learning(&lease, at() + 66).unwrap();
    assert_eq!(
        s.learning_job(failed["id"].as_str().unwrap())
            .unwrap()
            .unwrap()
            .state,
        "failed"
    );
    assert!(s.claim_learning(at() + 200).unwrap().is_none());
    let db = rusqlite::Connection::open(dir.path().join("synthetic.sqlite3")).unwrap();
    let retained:i64=db.query_row("SELECT count(*) FROM extraction_jobs WHERE state IN ('done','cancelled','failed') AND evidence_json!='[]'",[],|r|r.get(0)).unwrap();
    assert_eq!(retained, 0);
}
#[test]
fn model_confidence_and_invented_claims_never_authorize_promotion() {
    let (_dir, mut s) = setup();
    s.enqueue_learning(interaction("Meu PC tem 16 GB de RAM."))
        .unwrap();
    let lease = s.claim_learning(at() + 1).unwrap().unwrap();
    let mut fabricated = lease.claims.clone();
    fabricated[0].confidence = 1.0;
    fabricated[0].content = "Invented: 64 GB".into();
    assert!(s.complete_learning(&lease, &fabricated, at() + 2).is_err());
    assert!(s.list("lucille", "atlas").unwrap().is_empty());
}
#[test]
fn correction_aliases_replace_only_same_equipment_and_suspend_stale_context() {
    let (_dir, mut s) = setup();
    learn(&mut s, "Meu PC tem 16 GB de RAM.");
    let first = s.list("lucille", "atlas").unwrap().pop().unwrap();
    let old = approve(&mut s, &first);
    learn(&mut s, "Meu outro computador tem 8 GB de RAM.");
    let other = s
        .list("lucille", "atlas")
        .unwrap()
        .into_iter()
        .find(|m| m.input.content.contains("8 GB"))
        .unwrap();
    approve(&mut s, &other);
    learn(&mut s, "Corrigindo: agora instalei 32 GB de RAM.");
    let replacement = s
        .list("lucille", "atlas")
        .unwrap()
        .into_iter()
        .find(|m| m.state == Status::Conflict)
        .unwrap();
    let pending = s.context(req("Quanta RAM tem meu computador?")).unwrap();
    assert!(!pending.references.iter().any(|r| r.id == old.input.id));
    let new = approve(&mut s, &replacement);
    assert_eq!(
        s.list("lucille", "atlas")
            .unwrap()
            .into_iter()
            .find(|m| m.input.id == old.input.id)
            .unwrap()
            .state,
        Status::Obsolete
    );
    let context = s.context(req("Quanta RAM tem meu computador?")).unwrap();
    assert!(context.references.iter().any(|r| r.id == new.input.id));
    assert!(!context.references.iter().any(|r| r.id == old.input.id));
    assert!(!context
        .memories
        .iter()
        .any(|m| m.input.content.contains("8 GB")));
    assert!(s
        .context(req("Quanta RAM tem meu outro computador?"))
        .unwrap()
        .memories
        .iter()
        .any(|m| m.input.content.contains("8 GB")));
}
#[test]
fn preferences_consolidate_facets_with_provenance_undo_and_no_scope_merge() {
    let (_dir, mut s) = setup();
    learn(&mut s, "Eu gosto de RPG tático.");
    learn(
        &mut s,
        "Prefiro jogos de estratégia por turnos com classes.",
    );
    learn(&mut s, "Curto RPGs táticos com evolução de personagem.");
    let details = s.memory_details("lucille", "atlas").unwrap();
    assert_eq!(details.len(), 1);
    assert_eq!(details[0].provenance.len(), 3);
    assert!(
        details[0].memory.input.content.contains("classes")
            && details[0].memory.input.content.contains("evolução")
    );
    let m = &details[0].memory;
    let restored = s
        .undo_consolidation(&m.input.id, "lucille", "atlas", m.version, 1, &permit())
        .unwrap();
    assert!(!restored.input.content.contains("classes"));
    let mut input = interaction("Eu gosto de RPG tático.");
    input.project = "boreal".into();
    s.enqueue_learning(input).unwrap();
    let lease = s.claim_learning(at() + 1).unwrap().unwrap();
    s.complete_learning(&lease, &lease.claims, at() + 2)
        .unwrap();
    assert_eq!(s.list("lucille", "boreal").unwrap().len(), 1);
}
#[test]
fn expiry_does_not_erase_durable_facts_and_irrelevant_preferences_are_not_injected() {
    let (_dir, mut s) = setup();
    learn(&mut s, "Prefiro café sem açúcar.");
    learn(&mut s, "Meu PC tem 16 GB de RAM.");
    learn(&mut s, "Preciso revisar o relatório amanhã.");
    for m in s
        .list("lucille", "atlas")
        .unwrap()
        .into_iter()
        .filter(|m| m.state == Status::Candidate)
    {
        approve(&mut s, &m);
    }
    assert!(s
        .context(req("Qual é a capital da França?"))
        .unwrap()
        .memories
        .is_empty());
    let old = s
        .context_at(req("O que você lembra sobre mim?"), None, at() + 10 * 86400)
        .unwrap();
    assert!(old.memories.iter().any(|m| m.input.kind == Kind::Hardware));
    assert!(!old.memories.iter().any(|m| m.input.kind == Kind::Task));
    assert!(s
        .list("lucille", "atlas")
        .unwrap()
        .iter()
        .any(|m| m.input.kind == Kind::Task));
}
#[test]
fn deletion_blocks_replay_and_related_automatic_relearning_without_retaining_content() {
    let (_dir, mut s) = setup();
    let input = interaction("Prefiro café sem açúcar.");
    s.enqueue_learning(input.clone()).unwrap();
    let lease = s.claim_learning(at() + 1).unwrap().unwrap();
    s.complete_learning(&lease, &lease.claims, at() + 2)
        .unwrap();
    let m = s.list("lucille", "atlas").unwrap().pop().unwrap();
    s.change(&m.input.id, "lucille", "atlas", m.version, Action::Delete)
        .unwrap();
    assert_eq!(s.enqueue_learning(input).unwrap()["state"], "done");
    learn(&mut s, "I prefer coffee without sugar.");
    assert!(s.list("lucille", "atlas").unwrap().is_empty());
    assert_eq!(s.deleted_memories("lucille", "atlas").unwrap().len(), 1);
}
fn space(revision: &str) -> EmbeddingSpace {
    EmbeddingSpace {
        model: "synthetic-3d".into(),
        revision: revision.into(),
        fingerprint: "a".repeat(64),
        dimension: 3,
        normalization: "l2".into(),
        pooling: "mean".into(),
        document_prefix: "document: ".into(),
        query_prefix: "query: ".into(),
        context_tokens: 1024,
        index_version: 1,
    }
}
#[test]
fn hybrid_index_never_mixes_model_spaces_stale_versions_or_deleted_memories() {
    let (_dir, mut s) = setup();
    learn(&mut s, "Prefiro café sem açúcar.");
    let m = s.list("lucille", "atlas").unwrap().pop().unwrap();
    let spec = space("one");
    s.configure_embedding_space(&spec, &permit()).unwrap();
    s.put_memory_vector(&m.input.id, m.version, &spec, &[1.0, 0.0, 0.0], &permit())
        .unwrap();
    let query = QueryVector {
        space: spec.clone(),
        vector: vec![1.0, 0.0, 0.0],
    };
    assert_eq!(
        s.context_hybrid(req("Recomende uma bebida personalizada"), Some(&query))
            .unwrap()
            .memories
            .len(),
        1
    );
    let new = space("two");
    s.configure_embedding_space(&new, &permit()).unwrap();
    assert!(s
        .context_hybrid(req("Recomende uma bebida personalizada"), Some(&query))
        .unwrap()
        .memories
        .is_empty());
    assert!(s
        .put_memory_vector(&m.input.id, m.version, &spec, &[1.0, 0.0, 0.0], &permit())
        .is_err());
    assert!(!s
        .context(req("Qual café eu prefiro sem açúcar?"))
        .unwrap()
        .memories
        .is_empty());
    s.change(&m.input.id, "lucille", "atlas", m.version, Action::Delete)
        .unwrap();
    assert!(s
        .context_hybrid(req("Qual café eu prefiro?"), Some(&query))
        .unwrap()
        .memories
        .is_empty());
}
#[test]
fn phase4_backup_retains_pending_jobs_and_metadata_atomically() {
    let (_dir, mut s) = setup();
    learn(&mut s, "Prefiro café sem açúcar.");
    s.enqueue_learning(interaction("Meu PC tem 16 GB de RAM."))
        .unwrap();
    let backup = s.export_backup(&permit()).unwrap();
    assert!(backup.contains("\"schema\":5"));
    let (_target, mut restored) = setup();
    restored.restore_isolated(&backup, &permit()).unwrap();
    assert_eq!(
        restored.memory_details("lucille", "atlas").unwrap()[0]
            .provenance
            .len(),
        1
    );
    let lease = restored.claim_learning(at() + 1).unwrap().unwrap();
    restored
        .complete_learning(&lease, &lease.claims, at() + 2)
        .unwrap();
    assert_eq!(restored.list("lucille", "atlas").unwrap().len(), 2);
}

#[test]
fn retrieval_benchmark_measures_relevance_injection_scope_and_budget() {
    let (_dir, mut s) = setup();
    for text in [
        "Prefiro café sem açúcar.",
        "Eu gosto de RPG tático.",
        "Meu PC tem 16 GB de RAM.",
        "Estou com 8 GB de RAM livres agora.",
        "O projeto Atlas utiliza SQLite.",
        "Meu objetivo é aprender Rust.",
        "Preciso revisar o relatório amanhã.",
    ] {
        learn(&mut s, text);
    }
    for m in s
        .list("lucille", "atlas")
        .unwrap()
        .into_iter()
        .filter(|m| m.state == Status::Candidate)
    {
        approve(&mut s, &m);
    }
    let fixtures: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/phase2-retrieval.json")).unwrap();
    let (mut tp, mut fp, mut missed, mut duplicates, mut scope_errors) = (0, 0, 0, 0, 0);
    let mut failures = vec![];
    for case in &fixtures {
        let context = s.context(req(case["query"].as_str().unwrap())).unwrap();
        let details = s.memory_details("lucille", "atlas").unwrap();
        let actual = context
            .memories
            .iter()
            .map(|m| {
                let p = &details
                    .iter()
                    .find(|d| d.memory.input.id == m.input.id)
                    .unwrap()
                    .quality
                    .as_ref()
                    .unwrap()
                    .property;
                p.split('.').next().unwrap().to_string()
            })
            .collect::<Vec<_>>();
        let expected = case["expected"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        tp += actual.iter().filter(|v| expected.contains(v)).count();
        fp += actual.iter().filter(|v| !expected.contains(v)).count();
        missed += expected.iter().filter(|v| !actual.contains(v)).count();
        duplicates += actual.len()
            - actual
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len();
        scope_errors += context
            .memories
            .iter()
            .filter(|m| m.input.project != "atlas" || m.input.scope != "lucille")
            .count();
        if actual.iter().any(|v| !expected.contains(v))
            || expected.iter().any(|v| !actual.contains(v))
        {
            failures.push(
                serde_json::json!({"query":case["query"],"expected":expected,"actual":actual}),
            );
        }
    }
    let mut small = req("O que você lembra?");
    small.token_budget = 600;
    let budget = s.context(small).unwrap();
    assert!(budget.used_upper_bound <= 600);
    let mut other = req("O que você lembra?");
    other.project = "boreal".into();
    assert!(s.context(other).unwrap().memories.is_empty());
    let precision = tp as f64 / (tp + fp).max(1) as f64;
    let recall = tp as f64 / (tp + missed).max(1) as f64;
    let report = serde_json::json!({"cases":fixtures.len(),"true_positive":tp,"irrelevant_injected":fp,"missed":missed,"retrieval_precision":precision,"retrieval_recall":recall,"duplicates":duplicates,"scope_errors":scope_errors,"failures":failures});
    if let Some(path) = std::env::var_os("LUCILLE_PHASE2_RETRIEVAL_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    }
    assert!(
        precision >= 0.9 && recall >= 0.8 && duplicates == 0 && scope_errors == 0,
        "{report}"
    );
}

#[test]
fn queue_database_failure_rolls_back_and_two_connections_cannot_share_a_lease() {
    let (dir, mut s) = setup();
    let path = dir.path().join("synthetic.sqlite3");
    s.enqueue_learning(interaction("Meu PC tem 16 GB de RAM."))
        .unwrap();
    let mut second = Store::open(&path).unwrap();
    let lease = s.claim_learning(at() + 1).unwrap().unwrap();
    assert!(second.claim_learning(at() + 2).unwrap().is_none());
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TRIGGER synthetic_fail BEFORE INSERT ON memory_quality BEGIN SELECT RAISE(ABORT,'synthetic database failure'); END;").unwrap();
    assert!(s
        .complete_learning(&lease, &lease.claims, at() + 2)
        .is_err());
    assert!(s.list("lucille", "atlas").unwrap().is_empty());
    assert_eq!(s.learning_job(&lease.id).unwrap().unwrap().state, "running");
    db.execute_batch("DROP TRIGGER synthetic_fail;").unwrap();
    s.fail_learning(&lease, at() + 3).unwrap();
    let lease = second.claim_learning(at() + 10).unwrap().unwrap();
    assert!(second
        .complete_learning(&lease, &lease.claims, at() + 11)
        .unwrap());
    assert_eq!(s.list("lucille", "atlas").unwrap().len(), 1);
}

#[test]
fn queue_bounds_policy_disable_and_forged_lease_fail_closed() {
    let (_dir, mut s) = setup();
    let mut policy = s.learning_policy().unwrap();
    policy.max_pending = 1;
    s.set_learning_policy(policy.clone(), &permit()).unwrap();
    s.enqueue_learning(interaction("Prefiro café sem açúcar."))
        .unwrap();
    assert!(s
        .enqueue_learning(interaction("Meu PC tem 16 GB de RAM."))
        .is_err());
    let lease = s.claim_learning(at() + 1).unwrap().unwrap();
    let mut forged = lease.clone();
    forged.project = "boreal".into();
    assert!(s
        .complete_learning(&forged, &forged.claims, at() + 2)
        .is_err());
    policy.enabled = false;
    s.set_learning_policy(policy, &permit()).unwrap();
    assert!(!s
        .complete_learning(&lease, &lease.claims, at() + 3)
        .unwrap());
    assert!(s.list("lucille", "atlas").unwrap().is_empty());
}

#[test]
fn current_correction_precedes_async_write_and_negated_preferences_abstain() {
    let (_dir, mut s) = setup();
    learn(&mut s, "Meu PC tem 16 GB de RAM.");
    let candidate = s.list("lucille", "atlas").unwrap().pop().unwrap();
    approve(&mut s, &candidate);
    assert!(s
        .context(req("Corrigindo: agora instalei 32 GB de RAM."))
        .unwrap()
        .memories
        .is_empty());
    for text in [
        "Não prefiro café sem açúcar.",
        "Eu não gosto de café com açúcar.",
        "Não prefiro respostas em português.",
    ] {
        assert!(extract(text).is_empty());
    }
    assert_eq!(s.list("lucille", "atlas").unwrap().len(), 1);
}

#[test]
fn embedding_client_must_preserve_prefix_identity_and_version() {
    use std::cell::RefCell;
    struct Client {
        spec: EmbeddingSpace,
        seen: RefCell<Vec<String>>,
        drift: bool,
    }
    impl EmbeddingClient for Client {
        fn manifest(&self) -> Result<EmbeddingSpace> {
            let mut s = self.spec.clone();
            if self.drift && !self.seen.borrow().is_empty() {
                s.revision = "changed".into()
            }
            Ok(s)
        }
        fn encode(&self, text: &str) -> Result<Vec<f32>> {
            self.seen.borrow_mut().push(text.into());
            Ok(vec![1.0, 0.0, 0.0])
        }
    }
    let spec = space("one");
    let client = Client {
        spec: spec.clone(),
        seen: Default::default(),
        drift: false,
    };
    encode_query(&client, &spec, "synthetic question").unwrap();
    assert_eq!(client.seen.borrow()[0], "query: synthetic question");
    let drift = Client {
        spec: spec.clone(),
        seen: Default::default(),
        drift: true,
    };
    assert!(encode_query(&drift, &spec, "synthetic question").is_err());
    let (_dir, mut s) = setup();
    learn(&mut s, "Prefiro café sem açúcar.");
    let m = s.list("lucille", "atlas").unwrap().pop().unwrap();
    s.configure_embedding_space(&spec, &permit()).unwrap();
    s.embed_memory(&m.input.id, "lucille", "atlas", &client, &spec, &permit())
        .unwrap();
    assert!(client.seen.borrow()[1].starts_with("document: "));
    s.change(
        &m.input.id,
        "lucille",
        "atlas",
        m.version,
        Action::Update {
            content: "Prefiro café com açúcar.".into(),
            fact_key: m.input.fact_key.clone(),
        },
    )
    .unwrap();
    assert!(s
        .put_memory_vector(&m.input.id, m.version, &spec, &[1.0, 0.0, 0.0], &permit())
        .is_err());
    assert!(s
        .context_hybrid(
            req("Recomende uma bebida"),
            Some(&QueryVector {
                space: spec.clone(),
                vector: vec![1.0, 0.0, 0.0]
            })
        )
        .unwrap()
        .memories
        .is_empty());
    let mut nomic = spec;
    nomic.model = "nomic-ai/nomic-embed-text-v1.5".into();
    nomic.dimension = 768;
    assert!(nomic.validate().is_err());
    nomic.document_prefix = "search_document: ".into();
    nomic.query_prefix = "search_query: ".into();
    assert!(nomic.validate().is_ok());
}

#[test]
fn malformed_learning_backup_rolls_back_without_erasing_current_memory() {
    let (_dir, mut s) = setup();
    learn(&mut s, "Prefiro café sem açúcar.");
    let before = s.list("lucille", "atlas").unwrap();
    let mut backup: serde_json::Value =
        serde_json::from_str(&s.export_backup(&permit()).unwrap()).unwrap();
    backup["intelligent"]["tables"]["learning_policy"][0][1] =
        serde_json::json!("{\"enabled\":true,\"auto_kinds\":[\"hardware\"]}");
    assert!(s.restore_isolated(&backup.to_string(), &permit()).is_err());
    assert_eq!(
        s.list("lucille", "atlas").unwrap()[0].input.content,
        before[0].input.content
    );
}
