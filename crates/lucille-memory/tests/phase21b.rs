use lucille_memory::{intelligent::*, llm::*, semantic::*, *};
fn permit() -> ConfigurationPermit {
    ConfigurationPermit::explicit_settings_action(true).unwrap()
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
fn setup() -> (tempfile::TempDir, Store) {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::open(&d.path().join("synthetic.sqlite3")).unwrap();
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
    (d, s)
}
fn input(text: &str) -> Interaction {
    Interaction {
        conversation_id: "synthetic".into(),
        message_id: uuid::Uuid::new_v4().to_string(),
        scope: "lucille".into(),
        project: "atlas".into(),
        mode: Mode::Chat,
        role: "user".into(),
        user_text: text.into(),
    }
}
fn learn(s: &mut Store, text: &str) {
    s.enqueue_learning(input(text)).unwrap();
    if let Some(l) = s.claim_learning(now() + 1).unwrap() {
        s.complete_learning(&l, &l.claims, now() + 2).unwrap();
    }
}
fn approve(s: &mut Store, m: &Memory) {
    s.change(&m.input.id, "lucille", "atlas", m.version, Action::Approve)
        .unwrap();
}
fn request(q: &str) -> TurnRequest {
    TurnRequest {
        conversation_id: "synthetic".into(),
        message_id: uuid::Uuid::new_v4().to_string(),
        scope: "lucille".into(),
        project: "atlas".into(),
        mode: Mode::Chat,
        query: q.into(),
        token_budget: 8192,
    }
}
#[test]
fn historical_misses_are_regressions_not_unseen_data() {
    for text in [
        "No café, minha escolha continua sendo zero açúcar.",
        "When I pick games, I go for turn-based squad tactics.",
        "Eu prefiro campanhas em que cada decisão altera a história.",
    ] {
        let c = extract(text);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].kind, Kind::Preference);
        assert!(text.contains(&c[0].evidence));
    }
}
#[test]
fn device_evolution_keeps_work_laptop_and_hypothesis_separate() {
    let (_d, mut s) = setup();
    for text in [
        "Meu computador principal tem 16 GB de RAM.",
        "Meu notebook do trabalho tem 8 GB de RAM.",
    ] {
        learn(&mut s, text);
        let m = s
            .list("lucille", "atlas")
            .unwrap()
            .into_iter()
            .find(|m| m.state == Status::Candidate)
            .unwrap();
        approve(&mut s, &m);
    }
    assert!(s
        .context(request("Atualizei o computador principal para 32 GB."))
        .unwrap()
        .memories
        .is_empty());
    learn(&mut s, "Atualizei o computador principal para 32 GB.");
    let m = s
        .list("lucille", "atlas")
        .unwrap()
        .into_iter()
        .find(|m| m.state == Status::Conflict)
        .unwrap();
    approve(&mut s, &m);
    learn(&mut s, "Talvez eu aumente o notebook para 16 GB.");
    let records = s.memory_details("lucille", "atlas").unwrap();
    assert_eq!(
        records
            .iter()
            .filter(|d| d.memory.state == Status::Active
                && d.quality
                    .as_ref()
                    .is_some_and(|q| q.subject == "device.primary_computer" && q.value == "32_gb"))
            .count(),
        1
    );
    assert_eq!(
        records
            .iter()
            .filter(|d| d.memory.state == Status::Active
                && d.quality
                    .as_ref()
                    .is_some_and(|q| q.subject == "device.work_laptop" && q.value == "8_gb"))
            .count(),
        1
    );
    assert!(records
        .iter()
        .any(|d| d.memory.input.kind == Kind::Hypothesis && d.memory.state == Status::Candidate));
    assert!(!s
        .context(request("Quanta RAM tem meu computador principal?"))
        .unwrap()
        .memories
        .iter()
        .any(|m| m.input.content.contains("16 GB")));
    // Capacity without a property is intentionally not invented as installed RAM.
    assert!(extract("Meu notebook do trabalho tem 8 GB.").is_empty());
}
#[test]
fn unspecified_correction_resolves_only_a_unique_prior_capacity() {
    for ambiguous in [false, true] {
        let (_d, mut s) = setup();
        learn(&mut s, "Meu computador principal tem 16 GB de RAM.");
        let m = s.list("lucille", "atlas").unwrap().pop().unwrap();
        approve(&mut s, &m);
        if ambiguous {
            learn(&mut s, "Meu notebook do trabalho tem 16 GB de RAM.");
            let m = s
                .list("lucille", "atlas")
                .unwrap()
                .into_iter()
                .find(|m| m.state == Status::Candidate)
                .unwrap();
            approve(&mut s, &m);
        }
        assert!(s
            .context(request("Eu não tenho mais 16 GB, agora são 32 GB."))
            .unwrap()
            .memories
            .is_empty());
        learn(&mut s, "Eu não tenho mais 16 GB, agora são 32 GB.");
        let details = s.memory_details("lucille", "atlas").unwrap();
        let m = details
            .iter()
            .find(|d| d.memory.state != Status::Active)
            .unwrap();
        if ambiguous {
            assert_eq!(m.quality.as_ref().unwrap().subject, "device.unspecified");
            assert!(s
                .change(
                    &m.memory.input.id,
                    "lucille",
                    "atlas",
                    m.memory.version,
                    Action::Approve
                )
                .is_err());
            assert!(s
                .context(request("O que você lembra?"))
                .unwrap()
                .memories
                .is_empty());
        } else {
            assert_eq!(
                m.quality.as_ref().unwrap().subject,
                "device.primary_computer"
            );
            approve(&mut s, &m.memory);
            assert_eq!(
                s.list("lucille", "atlas")
                    .unwrap()
                    .iter()
                    .filter(|m| m.state == Status::Active)
                    .count(),
                1
            );
        }
    }
}
#[test]
fn coffee_withdrawal_and_completed_goals_suspend_old_context_until_review() {
    let (_d, mut s) = setup();
    learn(&mut s, "Prefiro café sem açúcar.");
    assert!(s
        .context(request("Não gosto mais de café."))
        .unwrap()
        .memories
        .is_empty());
    learn(&mut s, "Não gosto mais de café.");
    let m = s
        .list("lucille", "atlas")
        .unwrap()
        .into_iter()
        .find(|m| m.state == Status::Conflict)
        .unwrap();
    approve(&mut s, &m);
    let c = s.context(request("Qual café eu prefiro?")).unwrap();
    assert_eq!(c.memories.len(), 1);
    assert!(c.memories[0].input.content.contains("Não gosta"));
    learn(&mut s, "Meu objetivo é estudar Rust.");
    let m = s
        .list("lucille", "atlas")
        .unwrap()
        .into_iter()
        .find(|m| m.input.kind == Kind::Goal)
        .unwrap();
    approve(&mut s, &m);
    learn(&mut s, "Concluí meu objetivo de estudar Rust.");
    let m = s
        .list("lucille", "atlas")
        .unwrap()
        .into_iter()
        .find(|m| m.state == Status::Conflict)
        .unwrap();
    approve(&mut s, &m);
    assert!(!s
        .context(request("Ajude meu objetivo Rust"))
        .unwrap()
        .memories
        .iter()
        .any(|m| m.input.kind == Kind::Goal));
    assert!(s
        .context(request("Qual status do meu objetivo Rust?"))
        .unwrap()
        .memories
        .iter()
        .any(|m| m.input.content.contains("concluído")));
}
#[test]
fn unsupported_attribution_negation_and_fiction_still_abstain() {
    for text in [
        "Meu amigo tem 64 GB de RAM.",
        "He said I prefer tactical RPG games.",
        "Eu prefiro café sem açúcar mas isso não é verdade.",
        "Meu colega prefere café sem açúcar.",
        "Estou escrevendo uma história em que sou médico.",
        "Não prefiro café sem açúcar.",
        "My friend says I prefer turn-based squad tactics.",
        "Imagine que eu prefiro campanhas em que cada decisão altera a história.",
        "Meu computador e meu notebook de trabalho têm 16 GB de RAM.",
    ] {
        assert!(extract(text).is_empty(), "{text}");
    }
    let h = extract("Talvez eu compre uma RTX 5090.");
    assert_eq!(h.len(), 1);
    assert_eq!(h[0].kind, Kind::Hypothesis);
}
#[test]
fn extended_claims_still_require_full_grounding_and_exact_scope() {
    let text = "No café, minha escolha continua sendo zero açúcar.";
    let c = extract(text).pop().unwrap();
    let output = serde_json::json!({"proposals":[{"kind":c.kind,"content":c.content,"entity":c.subject,"property":c.property,"value":c.value,"scope":"lucille","project":"atlas","temporal":c.temporal,"evidence":c.evidence,"correction":c.correction,"hypothesis":false,"confidence":1.0,"relevance":1.0}]});
    assert_eq!(
        validate(text, "lucille", "atlas", &output.to_string())
            .unwrap()
            .len(),
        1
    );
    assert!(validate(text, "lucille", "boreal", &output.to_string()).is_err());
    assert!(validate(
        "Minha amiga disse: No café, minha escolha continua sendo zero açúcar.",
        "lucille",
        "atlas",
        &output.to_string()
    )
    .is_err());
}

fn interval(k: usize, n: usize) -> serde_json::Value {
    if n == 0 {
        return serde_json::Value::Null;
    }
    let z = 1.96;
    let n = n as f64;
    let p = k as f64 / n;
    let d = 1. + z * z / n;
    let center = (p + z * z / (2. * n)) / d;
    let half = z * ((p * (1. - p) + z * z / (4. * n)) / n).sqrt() / d;
    serde_json::json!([center - half, center + half])
}
#[test]
fn expanded_retrieval_uses_same_24_queries_with_scope_and_uncertainty() {
    let (_d, mut s) = setup();
    let spec = EmbeddingSpace {
        model: "synthetic-9d".into(),
        revision: "handcrafted-not-nomic".into(),
        fingerprint: "d".repeat(64),
        dimension: 9,
        normalization: "l2".into(),
        pooling: "mean".into(),
        document_prefix: "doc: ".into(),
        query_prefix: "query: ".into(),
        context_tokens: 1024,
        index_version: 1,
    };
    s.configure_embedding_space(&spec, &permit()).unwrap();
    let concepts = [
        ("bicycle", "two wheels"),
        ("telescope", "night sky instrument"),
        ("orchard", "fruit trees"),
        ("violin", "bowed strings"),
        ("kayak", "paddled watercraft"),
        ("volcano", "erupting mountain"),
        ("library", "book collection"),
        ("windmill", "wind powered blades"),
    ];
    let mut ids = vec![];
    for (i, (name, _)) in concepts.iter().enumerate() {
        let c = Candidate {
            id: uuid::Uuid::new_v4().to_string(),
            scope: "lucille".into(),
            project: "atlas".into(),
            kind: Kind::Fact,
            content: format!("Fictional {name} is teal."),
            fact_key: format!("fixture.{name}"),
            source: Source::Manual,
            source_id: "synthetic".into(),
        };
        let m = s.create_candidate(c).unwrap();
        approve(&mut s, &m);
        let m = s
            .list("lucille", "atlas")
            .unwrap()
            .into_iter()
            .find(|v| v.input.id == m.input.id)
            .unwrap();
        let mut v = vec![0.; 9];
        v[i] = 1.;
        s.put_memory_vector(&m.input.id, m.version, &spec, &v, &permit())
            .unwrap();
        ids.push(m.input.id);
    }
    let mut report = serde_json::Map::new();
    for (mode, lw, sw) in [("fts5", 1., 0.), ("semantic", 0., 1.), ("hybrid", 0.7, 0.3)] {
        let mut p = s.learning_policy().unwrap();
        p.lexical_weight = lw;
        p.semantic_weight = sw;
        s.set_learning_policy(p, &permit()).unwrap();
        let (mut tp, mut fp, mut missed) = (0, 0, 0);
        for (i, (exact, paraphrase)) in concepts.iter().enumerate() {
            for text in [*exact, *paraphrase] {
                let mut vector = vec![0.; 9];
                vector[i] = 1.;
                let q = QueryVector {
                    space: spec.clone(),
                    vector,
                };
                let c = s
                    .context_hybrid(request(text), if mode == "fts5" { None } else { Some(&q) })
                    .unwrap();
                tp += c.memories.iter().filter(|m| m.input.id == ids[i]).count();
                fp += c.memories.iter().filter(|m| m.input.id != ids[i]).count();
                if !c.memories.iter().any(|m| m.input.id == ids[i]) {
                    missed += 1
                }
            }
            let mut v = vec![0.; 9];
            v[8] = 1.;
            let q = QueryVector {
                space: spec.clone(),
                vector: v,
            };
            assert!(s
                .context_hybrid(
                    request(&format!("unrelated arithmetic {i}")),
                    if mode == "fts5" { None } else { Some(&q) }
                )
                .unwrap()
                .memories
                .is_empty());
        }
        report.insert(mode.into(),serde_json::json!({"queries":24,"tp":tp,"irrelevant":fp,"missed":missed,"precision":tp as f64/(tp+fp).max(1) as f64,"recall":tp as f64/16.,"precision_wilson_95":interval(tp,tp+fp),"recall_wilson_95":interval(tp,16),"simulation":true}));
        assert_eq!(fp, 0);
    }
    let mut other = request("bicycle");
    other.project = "boreal".into();
    let q = QueryVector {
        space: spec.clone(),
        vector: vec![1., 0., 0., 0., 0., 0., 0., 0., 0.],
    };
    assert!(s
        .context_hybrid(other, Some(&q))
        .unwrap()
        .memories
        .is_empty());
    let mut small = request("O que você lembra?");
    small.token_budget = 600;
    assert!(s.context(small).unwrap().used_upper_bound <= 600);
    if let Some(path) = std::env::var_os("LUCILLE_PHASE21B_RETRIEVAL_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap()
    }
}

#[test]
fn frozen_final_evaluation_scores_typed_claims_and_scope_in_real_sqlite() {
    let path = std::env::var_os("LUCILLE_PHASE21B_EVAL_PATH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| {
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/phase21b-final.json")
        });
    let bytes = std::fs::read(path).unwrap();
    use sha2::Digest;
    let manifest: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/phase21b-final.manifest.json")).unwrap();
    assert_eq!(
        format!("{:x}", sha2::Sha256::digest(&bytes)),
        manifest["sha256"].as_str().unwrap()
    );
    let dataset: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let cases = dataset["cases"].as_array().unwrap();
    let mut reports = serde_json::Map::new();
    for (label, mode) in [
        ("rules", ExtractionMode::Rules),
        ("hybrid_empty_simulator_fallback", ExtractionMode::Hybrid),
    ] {
        let (mut tp, mut fp, mut missed, mut unsafe_writes) = (0, 0, 0, 0);
        let mut failures = vec![];
        let mut groups = std::collections::BTreeMap::<String, (usize, usize, usize)>::new();
        for case in cases {
            let (_d, mut s) = setup();
            let mut policy = s.learning_policy().unwrap();
            policy.extraction_mode = mode.clone();
            s.set_learning_policy(policy, &permit()).unwrap();
            s.enqueue_learning(input(case["text"].as_str().unwrap()))
                .unwrap();
            if let Some(lease) = s.claim_learning(now() + 1).unwrap() {
                s.complete_learning(&lease, &lease.claims, now() + 2)
                    .unwrap();
            } else if let Some(lease) = s.claim_llm_learning(now() + 1).unwrap() {
                s.complete_llm_learning(&lease, "{\"proposals\":[]}", now() + 2)
                    .unwrap();
            }
            let actual = s.memory_details("lucille", "atlas").unwrap();
            let expected = case["expected"].as_array().unwrap();
            let mut matched = std::collections::HashSet::new();
            let (mut ctp, mut cfp) = (0, 0);
            for d in &actual {
                let q = d.quality.as_ref().unwrap();
                let target = expected.iter().enumerate().find(|(i, e)| {
                    !matched.contains(i)
                        && serde_json::to_value(&d.memory.input.kind).unwrap() == e["kind"]
                        && q.subject == e["subject"].as_str().unwrap()
                        && e.get("property")
                            .is_none_or(|v| v.as_str() == Some(&q.property))
                        && e.get("property_prefix")
                            .is_none_or(|v| q.property.starts_with(v.as_str().unwrap()))
                        && e.get("value").is_none_or(|v| v.as_str() == Some(&q.value))
                        && e.get("temporal")
                            .is_none_or(|v| v.as_str() == Some(&q.temporal))
                        && e.get("correction")
                            .is_none_or(|v| v.as_bool() == Some(q.priority == 2))
                });
                let kind = serde_json::to_value(&d.memory.input.kind)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_string();
                if let Some((i, _)) = target {
                    groups.entry(format!("kind:{kind}")).or_default().0 += 1;
                    matched.insert(i);
                    ctp += 1;
                } else {
                    groups.entry(format!("kind:{kind}")).or_default().1 += 1;
                    cfp += 1;
                }
                assert_eq!(d.memory.input.scope, "lucille");
                assert_eq!(d.memory.input.project, "atlas");
            }
            for (i, e) in expected.iter().enumerate() {
                if !matched.contains(&i) {
                    groups
                        .entry(format!("kind:{}", e["kind"].as_str().unwrap()))
                        .or_default()
                        .2 += 1;
                }
            }
            let cmissed = expected.len() - matched.len();
            tp += ctp;
            fp += cfp;
            missed += cmissed;
            if expected.is_empty() {
                unsafe_writes += actual.len();
            }
            for key in [
                format!("language:{}", case["language"].as_str().unwrap()),
                format!("category:{}", case["category"].as_str().unwrap()),
            ] {
                let g = groups.entry(key).or_default();
                g.0 += ctp;
                g.1 += cfp;
                g.2 += cmissed;
            }
            if cfp > 0 || cmissed > 0 {
                failures
                    .push(serde_json::json!({"id":case["id"],"expected":expected,"actual":actual}));
            }
        }
        let precision = tp as f64 / (tp + fp).max(1) as f64;
        let recall = tp as f64 / (tp + missed).max(1) as f64;
        reports.insert(label.into(),serde_json::json!({"tp":tp,"false_positive":fp,"missed":missed,"critical_writes":unsafe_writes,"precision":precision,"recall":recall,"by_category_language_tp_fp_missed":groups,"failures":failures,"gate_passed":precision>=0.95&&recall>=0.8&&unsafe_writes==0}));
    }
    let report = serde_json::json!({"dataset_version":dataset["version"],"cases":cases.len(),"modes":reports,"llm_simulated_quality":null,"llm_simulated_reason":"Protocol fixture tested separately; no model-quality score from scripted responses","real_llm":null,"real_nomic":null});
    if let Some(path) = std::env::var_os("LUCILLE_PHASE21B_EVAL_OUT") {
        std::fs::write(path, serde_json::to_string_pretty(&report).unwrap()).unwrap()
    }
    assert!(
        report["modes"]
            .as_object()
            .unwrap()
            .values()
            .all(|v| v["gate_passed"].as_bool() == Some(true)),
        "{report}"
    );
    // Quality failure is retained in the report and blocks approval/default activation,
    // rather than relabelling data or making a real-model claim.
    assert!(report["cases"].as_u64().unwrap() >= 30);
    assert!(
        report["modes"]["rules"]["gate_passed"].as_bool().unwrap(),
        "{report}"
    );
}
