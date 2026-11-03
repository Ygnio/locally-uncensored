#[allow(dead_code)]
#[path = "fixtures/phase2-reference-extractor.rs"]
mod phase2_reference;
use lucille_memory::{intelligent::*, llm::*, semantic::*, *};
fn permit() -> ConfigurationPermit {
    ConfigurationPermit::explicit_settings_action(true).unwrap()
}
fn time() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
fn setup(mode: ExtractionMode) -> (tempfile::TempDir, Store) {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::open(&d.path().join("synthetic.sqlite3")).unwrap();
    s.configure_identity(Profile::fixture(), 0, &permit())
        .unwrap();
    s.set_learning_policy(
        LearningPolicy {
            enabled: true,
            extraction_mode: mode,
            ..Default::default()
        },
        &permit(),
    )
    .unwrap();
    (d, s)
}
fn input(text: &str) -> Interaction {
    Interaction {
        conversation_id: "test-conversation".into(),
        message_id: uuid::Uuid::new_v4().to_string(),
        scope: "lucille".into(),
        project: "atlas".into(),
        mode: Mode::Chat,
        role: "user".into(),
        user_text: text.into(),
    }
}
fn proposal(evidence: &str) -> serde_json::Value {
    serde_json::json!({"proposals":[{"kind":"preference","content":"Prefere café sem açúcar.","entity":"user","property":"coffee_sugar","value":"without_sugar","scope":"lucille","project":"atlas","temporal":"preference","evidence":evidence,"correction":false,"hypothesis":false,"confidence":0.99,"relevance":0.95}]})
}
#[test]
fn llm_queue_survives_reopen_hybrid_deduplicates_and_purges_source() {
    let (d, mut s) = setup(ExtractionMode::Hybrid);
    let interaction = input("Prefiro café sem açúcar.");
    s.enqueue_learning(interaction.clone()).unwrap();
    assert!(s.claim_learning(time() + 1).unwrap().is_none());
    drop(s);
    let mut s = Store::open(&d.path().join("synthetic.sqlite3")).unwrap();
    let lease = s.claim_llm_learning(time() + 1).unwrap().unwrap();
    assert_eq!(
        s.learning_input(&lease).unwrap().unwrap().0,
        interaction.user_text
    );
    assert!(s
        .complete_llm_learning(
            &lease,
            &proposal("Prefiro café sem açúcar").to_string(),
            time() + 2
        )
        .unwrap());
    assert_eq!(s.list("lucille", "atlas").unwrap().len(), 1);
    assert_eq!(s.list("lucille", "atlas").unwrap()[0].state, Status::Active);
    assert!(s.learning_input(&lease).unwrap().is_none());
    s.enqueue_learning(interaction).unwrap();
    assert!(s.claim_llm_learning(time() + 3).unwrap().is_none());
}
#[test]
fn source_qualifiers_cannot_be_cut_out_or_replaced_by_model_confidence() {
    for text in [
        "Eu não prefiro café sem açúcar.",
        "Meu amigo disse: Prefiro café sem açúcar.",
        "Ele escreveu: Prefiro café sem açúcar.",
        "Imagine que eu prefiro café sem açúcar.",
        "Resultado da ferramenta: Prefiro café sem açúcar.",
    ] {
        assert!(
            validate(
                text,
                "lucille",
                "atlas",
                &proposal("Prefiro café sem açúcar").to_string()
            )
            .is_err(),
            "{text}"
        );
    }
    let mut p = proposal("Prefiro café sem açúcar");
    p["proposals"][0]["scope"] = serde_json::json!("other");
    assert!(validate(
        "Prefiro café sem açúcar.",
        "lucille",
        "atlas",
        &p.to_string()
    )
    .is_err());
    p = proposal("Prefiro café sem açúcar");
    p["proposals"][0]["identity"] = serde_json::json!("override");
    assert!(validate(
        "Prefiro café sem açúcar.",
        "lucille",
        "atlas",
        &p.to_string()
    )
    .is_err());
    p = proposal("Prefiro café sem açúcar");
    p["proposals"][0]["value"] = serde_json::json!("with_sugar");
    assert!(validate(
        "Prefiro café sem açúcar.",
        "lucille",
        "atlas",
        &p.to_string()
    )
    .is_err());
}
#[test]
fn novel_related_preferences_remain_separate_candidates_and_keep_provenance() {
    let (_d, mut s) = setup(ExtractionMode::Llm);
    for (text, property, value) in [
        (
            "Gosto de montar personagens com várias classes.",
            "games_classes",
            "várias classes",
        ),
        (
            "Prefiro jogos com evolução profunda.",
            "games_progression",
            "evolução profunda",
        ),
    ] {
        s.enqueue_learning(input(text)).unwrap();
        let lease = s.claim_llm_learning(time() + 1).unwrap().unwrap();
        let evidence = text.trim_end_matches('.');
        let output = serde_json::json!({"proposals":[{"kind":"preference","content":evidence,"entity":"user","property":property,"value":value,"scope":"lucille","project":"atlas","temporal":"preference","evidence":evidence,"correction":false,"hypothesis":false,"confidence":1.0,"relevance":1.0}]});
        s.complete_llm_learning(&lease, &output.to_string(), time() + 2)
            .unwrap();
    }
    let records = s.memory_details("lucille", "atlas").unwrap();
    assert_eq!(records.len(), 2);
    assert!(records
        .iter()
        .all(|d| d.memory.state == Status::Candidate && d.provenance.len() == 1));
}
#[test]
fn cancelled_disabled_expired_llm_leases_cannot_commit_and_backup_roundtrips() {
    let (_d, mut s) = setup(ExtractionMode::Llm);
    s.enqueue_learning(input("Prefiro café sem açúcar."))
        .unwrap();
    let first = s.claim_llm_learning(time() + 1).unwrap().unwrap();
    let second = s.claim_llm_learning(time() + 32).unwrap().unwrap();
    assert!(!s
        .complete_llm_learning(
            &first,
            &proposal("Prefiro café sem açúcar").to_string(),
            time() + 33
        )
        .unwrap());
    let backup = s.export_backup(&permit()).unwrap();
    let (_d2, mut target) = setup(ExtractionMode::Rules);
    target.restore_isolated(&backup, &permit()).unwrap();
    assert!(target.claim_llm_learning(time() + 34).unwrap().is_some());
    s.cancel_learning(&second.id, &permit()).unwrap();
    assert!(!s
        .complete_llm_learning(
            &second,
            &proposal("Prefiro café sem açúcar").to_string(),
            time() + 33
        )
        .unwrap());
    assert!(s.learning_input(&second).unwrap().is_none());
    s.enqueue_learning(input("Prefiro café sem açúcar."))
        .unwrap();
    let lease = s.claim_llm_learning(time() + 1).unwrap().unwrap();
    let mut policy = s.learning_policy().unwrap();
    policy.enabled = false;
    s.set_learning_policy(policy, &permit()).unwrap();
    assert!(s.learning_input(&lease).unwrap().is_none());
    assert!(!s
        .complete_llm_learning(&lease, "{\"proposals\":[]}", time() + 2)
        .unwrap());
}
#[test]
fn synthetic_retrieval_ablation_compares_all_three_modes() {
    let (_d, mut s) = setup(ExtractionMode::Rules);
    let c = Candidate {
        id: uuid::Uuid::new_v4().to_string(),
        scope: "lucille".into(),
        project: "atlas".into(),
        kind: Kind::Fact,
        content: "Fictional bicycle is teal.".into(),
        fact_key: "bicycle.color".into(),
        source: Source::Manual,
        source_id: "fixture".into(),
    };
    let m = s.create_candidate(c).unwrap();
    let m = s
        .change(&m.input.id, "lucille", "atlas", m.version, Action::Approve)
        .unwrap()
        .unwrap();
    let spec = EmbeddingSpace {
        model: "synthetic-3d".into(),
        revision: "fixture-only".into(),
        fingerprint: "b".repeat(64),
        dimension: 3,
        normalization: "l2".into(),
        pooling: "mean".into(),
        document_prefix: "doc: ".into(),
        query_prefix: "query: ".into(),
        context_tokens: 1024,
        index_version: 1,
    };
    s.configure_embedding_space(&spec, &permit()).unwrap();
    s.put_memory_vector(&m.input.id, m.version, &spec, &[1., 0., 0.], &permit())
        .unwrap();
    let mut report = serde_json::Map::new();
    for (mode, lexical, semantic) in [
        ("lexical", 1., 0.),
        ("semantic", 0., 1.),
        ("hybrid", 0.7, 0.3),
    ] {
        let mut policy = s.learning_policy().unwrap();
        policy.lexical_weight = lexical;
        policy.semantic_weight = semantic;
        s.set_learning_policy(policy, &permit()).unwrap();
        let (mut tp, mut fp, mut missed) = (0, 0, 0);
        for (query, vector, expected) in [
            ("bicycle", vec![1., 0., 0.], true),
            ("two wheels", vec![1., 0., 0.], true),
            ("capital of France", vec![0., 1., 0.], false),
        ] {
            let request = TurnRequest {
                conversation_id: "test".into(),
                message_id: uuid::Uuid::new_v4().to_string(),
                scope: "lucille".into(),
                project: "atlas".into(),
                mode: Mode::Chat,
                query: query.into(),
                token_budget: 8192,
            };
            let query = QueryVector {
                space: spec.clone(),
                vector,
            };
            let context = s
                .context_hybrid(
                    request,
                    if mode == "lexical" {
                        None
                    } else {
                        Some(&query)
                    },
                )
                .unwrap();
            if expected {
                if context.memories.len() == 1 {
                    tp += 1
                } else {
                    missed += 1
                }
            } else {
                fp += context.memories.len()
            };
        }
        report.insert(mode.into(),serde_json::json!({"tp":tp,"false_positive":fp,"missed":missed,"precision":tp as f64/(tp+fp).max(1) as f64,"recall":tp as f64/2.0,"simulation":true}));
        assert_eq!(fp, 0);
    }
    assert_eq!(report["lexical"]["missed"], 1);
    assert_eq!(report["hybrid"]["missed"], 0);
    assert_eq!(report["semantic"]["missed"], 0);
    if let Some(path) = std::env::var_os("LUCILLE_PHASE21_ABLATION_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap()
    }
}

#[test]
fn frozen_new_evaluation_records_rule_baseline_without_claiming_real_llm_metrics() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/phase21-heldout.json")).unwrap();
    let (mut tp, mut fp, mut missed) = (0, 0, 0);
    let mut failures = vec![];
    let mut groups = std::collections::BTreeMap::<String, (usize, usize, usize)>::new();
    for case in &cases {
        let claims = extract(case["text"].as_str().unwrap());
        let actual = claims
            .first()
            .map(|c| serde_json::to_value(&c.kind).unwrap());
        let expected = case["expected"].as_str();
        let correct = expected.is_some() && actual.as_ref().and_then(|v| v.as_str()) == expected;
        let wrong = (!claims.is_empty()) && !correct;
        let missing = expected.is_some() && !correct;
        tp += usize::from(correct);
        fp += usize::from(wrong);
        missed += usize::from(missing);
        for key in [
            format!("language:{}", case["language"].as_str().unwrap()),
            format!("category:{}", expected.unwrap_or("abstain")),
        ] {
            let counts = groups.entry(key).or_default();
            counts.0 += usize::from(correct);
            counts.1 += usize::from(wrong);
            counts.2 += usize::from(missing);
        }
        if wrong || missing {
            failures.push(
                serde_json::json!({"id":case["id"],"expected":case["expected"],"actual":actual}),
            );
        }
    }
    let report = serde_json::json!({"cases":cases.len(),"dataset_sha256":"46130605357e9b797832f3479a14227742eb5cd240df58f9055df16dbbdf588f","phase2_rules":{"tp":tp,"false_positive":fp,"missed":missed,"precision":tp as f64/(tp+fp).max(1) as f64,"recall":tp as f64/(tp+missed).max(1) as f64,"groups_tp_fp_missed":groups,"failures":failures},"phase21_real_llm":null,"real_latency_ms":null,"real_ram_vram":null,"real_tokens":null,"reason":"No authorized running isolated Gemma endpoint; simulated protocol tests are not model quality"});
    if let Some(path) = std::env::var_os("LUCILLE_PHASE21_EVAL_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap()
    }
    // This measurement preserves baseline misses. It is not an assertion that
    // the baseline meets the new recall gate or that LLM inference occurred.
    assert_eq!(fp, 0, "{report}");
}

#[test]
fn forged_failure_count_mode_changes_and_terminal_cleanup_are_fenced() {
    let (d, mut s) = setup(ExtractionMode::Llm);
    s.enqueue_learning(input("Prefiro café sem açúcar."))
        .unwrap();
    let mut lease = s.claim_llm_learning(time() + 1).unwrap().unwrap();
    lease.attempts = i64::MAX;
    s.fail_learning(&lease, time() + 2).unwrap();
    assert_eq!(s.learning_job(&lease.id).unwrap().unwrap().state, "retry");
    let lease = s.claim_llm_learning(time() + 10).unwrap().unwrap();
    s.fail_learning(&lease, time() + 11).unwrap();
    let lease = s.claim_llm_learning(time() + 20).unwrap().unwrap();
    s.fail_learning(&lease, time() + 21).unwrap();
    assert_eq!(s.learning_job(&lease.id).unwrap().unwrap().state, "failed");
    let db = rusqlite::Connection::open(d.path().join("synthetic.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM learning_inputs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    s.enqueue_learning(input("Prefiro café sem açúcar."))
        .unwrap();
    let mut policy = s.learning_policy().unwrap();
    policy.extraction_mode = ExtractionMode::Rules;
    s.set_learning_policy(policy, &permit()).unwrap();
    assert!(s.claim_llm_learning(time() + 30).unwrap().is_none());
    assert_eq!(
        db.query_row("SELECT count(*) FROM learning_inputs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn llm_commit_database_failure_retains_retry_source_without_partial_memory() {
    let (d, mut s) = setup(ExtractionMode::Llm);
    s.enqueue_learning(input("Prefiro café sem açúcar."))
        .unwrap();
    let lease = s.claim_llm_learning(time() + 1).unwrap().unwrap();
    let db = rusqlite::Connection::open(d.path().join("synthetic.sqlite3")).unwrap();
    db.execute_batch("CREATE TRIGGER synthetic_llm_failure BEFORE INSERT ON memory_quality BEGIN SELECT RAISE(ABORT,'synthetic'); END;").unwrap();
    assert!(s
        .complete_llm_learning(
            &lease,
            &proposal("Prefiro café sem açúcar").to_string(),
            time() + 2
        )
        .is_err());
    assert!(s.list("lucille", "atlas").unwrap().is_empty());
    assert!(s.learning_input(&lease).unwrap().is_some());
    assert_eq!(s.learning_job(&lease.id).unwrap().unwrap().state, "running");
    db.execute_batch("DROP TRIGGER synthetic_llm_failure;")
        .unwrap();
    assert!(s
        .complete_llm_learning(
            &lease,
            &proposal("Prefiro café sem açúcar").to_string(),
            time() + 3
        )
        .unwrap());
    assert!(s.learning_input(&lease).unwrap().is_none());
}

#[test]
fn independent_postfreeze_rule_evaluation_preserves_failures() {
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/phase21-independent.json")).unwrap();
    let (mut tp, mut fp, mut missed) = (0, 0, 0);
    let mut failures = vec![];
    let mut groups = std::collections::BTreeMap::<String, (usize, usize, usize)>::new();
    for case in &cases {
        let claims = extract(case["text"].as_str().unwrap());
        let actual = claims
            .first()
            .map(|c| serde_json::to_value(&c.kind).unwrap());
        let expected = case["expected"].as_str();
        let correct = expected.is_some() && actual.as_ref().and_then(|v| v.as_str()) == expected;
        let wrong = !claims.is_empty() && !correct;
        let missing = expected.is_some() && !correct;
        tp += usize::from(correct);
        fp += usize::from(wrong);
        missed += usize::from(missing);
        for key in [
            format!("language:{}", case["language"].as_str().unwrap()),
            format!("category:{}", expected.unwrap_or("abstain")),
        ] {
            let g = groups.entry(key).or_default();
            g.0 += usize::from(correct);
            g.1 += usize::from(wrong);
            g.2 += usize::from(missing);
        }
        if wrong || missing {
            failures.push(
                serde_json::json!({"id":case["id"],"expected":case["expected"],"actual":actual}),
            );
        }
    }
    let (mut baseline_tp, mut baseline_fp, mut baseline_missed) = (0, 0, 0);
    for case in &cases {
        let claims = phase2_reference::extract(case["text"].as_str().unwrap());
        let actual = claims
            .first()
            .map(|c| serde_json::to_value(&c.kind).unwrap());
        let expected = case["expected"].as_str();
        let correct = expected.is_some() && actual.as_ref().and_then(|v| v.as_str()) == expected;
        baseline_tp += usize::from(correct);
        baseline_fp += usize::from(!claims.is_empty() && !correct);
        baseline_missed += usize::from(expected.is_some() && !correct);
    }
    let report = serde_json::json!({"cases":cases.len(),"phase2_reference":{"commit":"c4a401b55d8f7231351039a744c861344c4a4cab","tp":baseline_tp,"false_positive":baseline_fp,"missed":baseline_missed,"precision":baseline_tp as f64/(baseline_tp+baseline_fp).max(1) as f64,"recall":baseline_tp as f64/(baseline_tp+baseline_missed).max(1) as f64},"rules":{"tp":tp,"false_positive":fp,"missed":missed,"precision":tp as f64/(tp+fp).max(1) as f64,"recall":tp as f64/(tp+missed).max(1) as f64,"by_language_and_category_tp_fp_missed":groups,"failures":failures},"real_llm":null,"real_nomic":null,"independent_from_development_examples":true});
    if let Some(path) = std::env::var_os("LUCILLE_PHASE21_INDEPENDENT_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap()
    }
    assert_eq!(fp, 0, "{report}");
}
#[test]
fn forwarded_and_role_instructions_never_enter_any_extractor_mode() {
    for text in [
        "Encaminhado: prefiro café com açúcar.",
        "Forwarded: I prefer coffee without sugar.",
        "Seu papel: eu prefiro café sem açúcar.",
        "Eu gosto de RPG tático, só que não.",
    ] {
        assert!(extract(text).is_empty(), "{text}");
        assert!(!eligible_source(text));
    }
}
