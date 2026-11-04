use lucille_memory::{intelligent::*, llm::ExtractionMode, *};
fn permit() -> ConfigurationPermit {
    ConfigurationPermit::explicit_settings_action(true).unwrap()
}
fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}
fn setup(auto: bool) -> (tempfile::TempDir, Store) {
    let d = tempfile::tempdir().unwrap();
    let mut s = Store::open(&d.path().join("synthetic.sqlite3")).unwrap();
    s.configure_identity(Profile::fixture(), 0, &permit())
        .unwrap();
    s.set_learning_policy(
        LearningPolicy {
            enabled: true,
            autonomous_preferences: auto,
            automatic_indexing: auto,
            extraction_mode: ExtractionMode::Llm,
            ..Default::default()
        },
        &permit(),
    )
    .unwrap();
    (d, s)
}
fn input(text: &str, id: &str) -> Interaction {
    Interaction {
        conversation_id: "conversation".into(),
        message_id: id.into(),
        scope: "lucille".into(),
        project: "demo".into(),
        mode: Mode::Chat,
        role: "user".into(),
        user_text: text.into(),
    }
}
fn output(text: &str, kind: &str) -> String {
    serde_json::json!({"version":3,"scope":"lucille","project":"demo","claims":[{"evidence":text.trim().trim_end_matches('.'),"category":kind}]}).to_string()
}
fn learn(s: &mut Store, text: &str, kind: &str) {
    s.enqueue_learning(input(text, &uuid::Uuid::new_v4().to_string()))
        .unwrap();
    if let Some(l) = s.claim_llm_learning(now() + 1).unwrap() {
        s.complete_llm_learning(&l, &output(text, kind), now() + 2)
            .unwrap();
    }
}
fn request(query: &str) -> TurnRequest {
    TurnRequest {
        conversation_id: "new-empty-session".into(),
        message_id: uuid::Uuid::new_v4().to_string(),
        scope: "lucille".into(),
        project: "demo".into(),
        mode: Mode::Chat,
        query: query.into(),
        token_budget: 8192,
    }
}

#[test]
fn typed_plan_and_fresh_evidence_revoke_deleted_authority() {
    let (_d, mut s) = setup(true);
    learn(&mut s, "Quero usar Rust.", "decision");
    let m = s.list("lucille", "demo").unwrap().pop().unwrap();
    s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
        .unwrap();
    let req = request("Que linguagem está vigente no meu projeto agora?");
    let ctx = s.context(req.clone()).unwrap();
    assert_eq!(ctx.facts.len(), 1);
    assert_eq!(ctx.facts[0].modality, "planned");
    assert_eq!(ctx.facts[0].review, "reviewed");
    assert_eq!(ctx.facts[0].evidence[0].text, "Quero usar Rust");
    assert_eq!(
        s.answer_evidence(&req, &ctx.references)
            .unwrap()
            .facts
            .len(),
        1
    );
    let m = &ctx.memories[0];
    s.change(&m.input.id, "lucille", "demo", m.version, Action::Delete)
        .unwrap();
    assert!(s
        .answer_evidence(&req, &ctx.references)
        .unwrap()
        .facts
        .is_empty());
}
#[test]
fn preference_paraphrase_survives_reopen_and_never_crosses_project() {
    let (d, mut s) = setup(true);
    learn(&mut s, "Prefiro café sem açúcar.", "preference");
    drop(s);
    let mut s = Store::open(&d.path().join("synthetic.sqlite3")).unwrap();
    let ctx = s
        .context(request("Como eu prefiro tomar o meu café?"))
        .unwrap();
    assert_eq!(ctx.facts.len(), 1);
    assert_eq!(ctx.facts[0].property, "coffee_sugar");
    assert_eq!(ctx.facts[0].value, "without_sugar");
    let mut foreign = request("Como eu prefiro tomar meu café no projeto demo2?");
    assert!(s.context(foreign.clone()).unwrap().memories.is_empty());
    foreign.project = "demo2".into();
    assert!(s.context(foreign).unwrap().memories.is_empty());
    assert!(s
        .context(request("Como meu amigo prefere tomar café?"))
        .unwrap()
        .memories
        .is_empty());
    assert!(s
        .context(request("Qual é a melhor receita de café?"))
        .unwrap()
        .memories
        .is_empty());
}
#[test]
fn new_safe_preferences_have_separate_properties_and_literal_sources() {
    let (_d, mut s) = setup(true);
    for (text, property, value) in [
        (
            "Eu gosto de criar personagens em jogos.",
            "games_character_creation",
            "positive",
        ),
        (
            "Eu prefiro jogos em que posso escolher classes.",
            "games_class_selection",
            "positive",
        ),
        (
            "Eu detesto interface poluída.",
            "interface_clutter",
            "negative",
        ),
        ("Minha bebida favorita é chá.", "favorite_beverage", "tea"),
        (
            "Eu prefiro respostas objetivas.",
            "communication_focus",
            "objective",
        ),
        (
            "Eu prefiro violão acústico.",
            "music_instrument",
            "acoustic_guitar",
        ),
    ] {
        learn(&mut s, text, "preference");
        let rows = s.memory_details("lucille", "demo").unwrap();
        let row = rows
            .iter()
            .find(|m| m.quality.as_ref().is_some_and(|q| q.property == property))
            .expect(text);
        assert_eq!(row.memory.state, Status::Active, "{text}");
        assert_eq!(row.quality.as_ref().unwrap().value, value);
        assert_eq!(row.provenance.len(), 1);
    }
    assert!(!s
        .memory_details("lucille", "demo")
        .unwrap()
        .iter()
        .any(|m| m
            .quality
            .as_ref()
            .is_some_and(|q| q.property == "communication_style")));
}
#[test]
fn source_grammar_accepts_pronouns_and_time_but_rejects_foreign_named_project() {
    for text in [
        "Na interface do meu projeto, eu prefiro o tema claro.",
        "Que linguagem está vigente no meu projeto agora?",
    ] {
        assert!(lucille_memory::dialogue::project_scope_matches(
            text, "demo"
        ));
    }
    for text in [
        "Projeto demo2 usa Rust.",
        "O projeto Atlas está em desenvolvimento.",
    ] {
        assert!(!lucille_memory::dialogue::project_scope_matches(
            text, "demo"
        ));
    }
}
#[test]
fn malformed_output_releases_owned_lease_and_retries_are_bounded() {
    let (d, mut s) = setup(true);
    s.enqueue_learning(input("Prefiro café sem açúcar.", "malformed"))
        .unwrap();
    let mut at = now() + 1;
    let mut old = None;
    for attempt in 1..=3 {
        let lease = s.claim_llm_learning(at).unwrap().expect("retry available");
        assert!(s.complete_llm_learning(&lease, "not json", at + 1).is_err());
        let db = rusqlite::Connection::open(d.path().join("synthetic.sqlite3")).unwrap();
        let (state, count, token): (String, i64, Option<String>) = db
            .query_row(
                "SELECT state,attempts,lease_token FROM extraction_jobs WHERE id=?1",
                [&lease.id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(count, attempt);
        assert_eq!(state, if attempt == 3 { "failed" } else { "retry" });
        assert!(token.is_none());
        old = Some(lease);
        at += 20;
    }
    assert!(s.claim_llm_learning(at).unwrap().is_none());
    assert!(!s
        .complete_llm_learning(
            &old.unwrap(),
            &output("Prefiro café sem açúcar.", "preference"),
            at
        )
        .unwrap());
    let db = rusqlite::Connection::open(d.path().join("synthetic.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM learning_inputs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(s.list("lucille", "demo").unwrap().is_empty());
}
#[test]
fn vector_collection_is_idempotent_and_preserves_canonical_history() {
    let (d, mut s) = setup(true);
    learn(&mut s, "Prefiro café sem açúcar.", "preference");
    let space = lucille_memory::semantic::EmbeddingSpace {
        model: "synthetic-gc".into(),
        revision: "1".into(),
        fingerprint: "b".repeat(64),
        dimension: 2,
        normalization: "l2".into(),
        pooling: "mean".into(),
        document_prefix: "doc: ".into(),
        query_prefix: "query: ".into(),
        context_tokens: 2048,
        index_version: 4,
    };
    s.configure_embedding_space(&space, &permit()).unwrap();
    let m = s.list("lucille", "demo").unwrap().pop().unwrap();
    s.put_memory_vector(&m.input.id, m.version, &space, &[1., 0.], &permit())
        .unwrap();
    learn(
        &mut s,
        "Na verdade, agora prefiro café com açúcar.",
        "preference",
    );
    let before = s.memory_details("lucille", "demo").unwrap();
    assert_eq!(
        before
            .iter()
            .filter(|m| m.memory.state == Status::Obsolete)
            .count(),
        1
    );
    let db = rusqlite::Connection::open(d.path().join("synthetic.sqlite3")).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM memory_vectors", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(s.prune_obsolete_vectors().unwrap(), 0);
    assert_eq!(s.prune_obsolete_vectors().unwrap(), 0);
    let ids = before
        .iter()
        .map(|m| m.memory.input.id.clone())
        .collect::<Vec<_>>();
    drop(s);
    let s = Store::open(&d.path().join("synthetic.sqlite3")).unwrap();
    assert_eq!(
        s.memory_details("lucille", "demo").unwrap().len(),
        before.len()
    );
    for id in ids {
        assert!(!s.history(&id).unwrap().is_empty());
    }
}

#[test]
fn future_choice_and_current_implementation_coexist_without_becoming_equivalent() {
    let (_d, mut s) = setup(true);
    learn(&mut s, "Meu projeto usa Rust.", "fact");
    let m = s.list("lucille", "demo").unwrap().pop().unwrap();
    s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
        .unwrap();
    learn(&mut s, "Quero usar Python.", "decision");
    let m = s
        .list("lucille", "demo")
        .unwrap()
        .into_iter()
        .find(|m| m.state == Status::Candidate)
        .unwrap();
    s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
        .unwrap();
    let details = s.memory_details("lucille", "demo").unwrap();
    assert_eq!(
        details
            .iter()
            .filter(|m| m.memory.state == Status::Active)
            .count(),
        2,
        "{details:?}"
    );
    let ctx = s
        .context(request("Qual linguagem usa meu projeto?"))
        .unwrap();
    assert!(ctx
        .facts
        .iter()
        .any(|f| f.value == "rust" && f.modality == "current_fact"));
    assert!(!ctx
        .facts
        .iter()
        .any(|f| f.value == "python" && f.modality == "current_fact"));
}
#[test]
fn historical_preference_is_explicitly_tagged_and_deleted_history_is_unavailable() {
    let (_d, mut s) = setup(true);
    learn(&mut s, "Prefiro café sem açúcar.", "preference");
    learn(
        &mut s,
        "Na verdade, agora prefiro café com açúcar.",
        "preference",
    );
    let ctx = s
        .context(request("Como eu prefiro tomar café agora?"))
        .unwrap();
    assert!(ctx
        .facts
        .iter()
        .all(|f| f.validity == "current" && f.value == "with_sugar"));
    let req = request("Como eu preferia tomar meu café antes?");
    let ctx = s.context(req.clone()).unwrap();
    let old = ctx
        .facts
        .iter()
        .find(|f| f.validity == "historical" && f.value == "without_sugar")
        .expect("explicit scoped history");
    assert!(s
        .answer_evidence(&req, &ctx.references)
        .unwrap()
        .facts
        .iter()
        .any(|f| f.id == old.id && f.validity == "historical"));
    s.change(&old.id, "lucille", "demo", old.version, Action::Delete)
        .unwrap();
    assert!(!s
        .context(req)
        .unwrap()
        .facts
        .iter()
        .any(|f| f.value == "without_sugar"));
}
#[test]
fn explicit_unknown_and_cancelled_project_keep_their_typed_modalities() {
    let (_d, mut s) = setup(true);
    for (text, category, property, modality) in [
        (
            "Ainda não decidi a linguagem do meu projeto.",
            "decision",
            "language",
            "explicitly_unknown",
        ),
        ("Cancelei meu projeto.", "project", "lifecycle", "cancelled"),
    ] {
        learn(&mut s, text, category);
        let m = s
            .list("lucille", "demo")
            .unwrap()
            .into_iter()
            .find(|m| m.state == Status::Candidate)
            .unwrap();
        s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
            .unwrap();
        let ctx = s
            .context(request("Qual é o status do meu projeto e sua linguagem?"))
            .unwrap();
        assert!(
            ctx.facts
                .iter()
                .any(|f| f.property == property && f.modality == modality),
            "{text}: {ctx:?}"
        );
    }
}

#[test]
fn rejected_candidate_never_becomes_a_historical_user_preference() {
    let (_d, mut s) = setup(false);
    learn(&mut s, "Prefiro café sem açúcar.", "preference");
    let m = s.list("lucille", "demo").unwrap().pop().unwrap();
    s.change(&m.input.id, "lucille", "demo", m.version, Action::Reject)
        .unwrap();
    assert!(s
        .context(request("Como eu preferia tomar meu café antes?"))
        .unwrap()
        .facts
        .is_empty());
}
#[test]
fn expanded_preferences_keep_literal_authorship_and_forgetting_revokes_late_output() {
    let (_d, mut s) = setup(true);
    learn(
        &mut s,
        "Eu gosto de jogos com criação de personagens e de escolher classes.",
        "preference",
    );
    let details = s.memory_details("lucille", "demo").unwrap();
    assert_eq!(details.len(), 2);
    assert!(details.iter().all(|m| m.memory.state == Status::Active));
    s.enqueue_learning(input(
        "Eu prefiro jogos em que posso escolher classes.",
        "late-class",
    ))
    .unwrap();
    let lease = s.claim_llm_learning(now() + 1).unwrap().unwrap();
    s.enqueue_learning(input(
        "Esqueça minha preferência por escolher classes.",
        "forget-class",
    ))
    .unwrap();
    assert!(!s
        .complete_llm_learning(
            &lease,
            &output(
                "Eu prefiro jogos em que posso escolher classes.",
                "preference"
            ),
            now() + 2
        )
        .unwrap());
    assert!(!s
        .memory_details("lucille", "demo")
        .unwrap()
        .iter()
        .any(|m| m
            .quality
            .as_ref()
            .is_some_and(|q| q.property == "games_class_selection")));
    assert!(s
        .memory_details("lucille", "demo")
        .unwrap()
        .iter()
        .any(|m| m
            .quality
            .as_ref()
            .is_some_and(|q| q.property == "games_character_creation")));
    for text in [
        "Meu filho gosta de criar personagens.",
        "Minha esposa prefere escolher classes.",
        "Talvez minha bebida favorita seja chá.",
    ] {
        assert!(
            lucille_memory::dialogue::extract(text, "demo").is_empty(),
            "{text}"
        );
    }
}

#[test]
fn bounded_typo_normalization_never_fuzzes_project_names() {
    let (_d, mut s) = setup(true);
    learn(&mut s, "Quero usar Rust.", "decision");
    let m = s.list("lucille", "demo").unwrap().pop().unwrap();
    s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
        .unwrap();
    assert_eq!(
        s.context(request("Qual linguageem escolhi para meu projeto?"))
            .unwrap()
            .facts
            .len(),
        1
    );
    assert!(s
        .context(request("Qual linguageem escolhi para o projeto demoo?"))
        .unwrap()
        .facts
        .is_empty());
}

#[test]
fn additional_claim_confirmation_is_scoped_fresh_and_does_not_relabel_retrieval() {
    use lucille_memory::context_facts::ClaimedField;
    let (_d, mut s) = setup(true);
    learn(&mut s, "Prefiro café sem açúcar.", "preference");
    let req = request("Minha bebida favorita é chá.");
    assert!(s.context(req.clone()).unwrap().references.is_empty());
    let claim = ClaimedField {
        entity: "user".into(),
        property: "coffee_sugar".into(),
        value: "without_sugar".into(),
    };
    let confirmed = s
        .answer_evidence_for_claims(&req, &[], &[claim.clone()])
        .unwrap();
    assert_eq!(confirmed.facts.len(), 1);
    let mut foreign = req.clone();
    foreign.project = "other".into();
    assert!(s
        .answer_evidence_for_claims(&foreign, &[], &[claim.clone()])
        .unwrap()
        .facts
        .is_empty());
    let f = &confirmed.facts[0];
    s.change(&f.id, "lucille", "demo", f.version, Action::Delete)
        .unwrap();
    assert!(s
        .answer_evidence_for_claims(&req, &[], &[claim])
        .unwrap()
        .facts
        .is_empty());
    assert!(s
        .answer_evidence_for_claims(
            &req,
            &[],
            &[ClaimedField {
                entity: "project.other".into(),
                property: "language".into(),
                value: "rust".into()
            }]
        )
        .is_err());
}
#[test]
fn requested_natural_preference_examples_are_grounded_without_adding_game_context() {
    let (_d, mut s) = setup(true);
    for (text, property, value) in [
        (
            "Adoro jogos com criação de personagens.",
            "games_character_creation",
            "positive",
        ),
        (
            "Não curto interfaces muito carregadas.",
            "interface_clutter",
            "negative",
        ),
        ("Minha bebida favorita é chá.", "favorite_beverage", "tea"),
        (
            "Prefiro respostas objetivas.",
            "communication_focus",
            "objective",
        ),
        (
            "Gosto de jogos em que posso escolher minha classe.",
            "games_class_selection",
            "positive",
        ),
    ] {
        learn(&mut s, text, "preference");
        assert!(
            s.memory_details("lucille", "demo")
                .unwrap()
                .iter()
                .any(|d| d.memory.state == Status::Active
                    && d.quality
                        .as_ref()
                        .is_some_and(|q| q.property == property && q.value == value)),
            "{text}"
        );
    }
    assert!(!lucille_memory::dialogue::extract(
        "Eu gosto de criar personagens para escrever romances.",
        "demo"
    )
    .iter()
    .any(|c| c.property == "games_character_creation"));
}
#[test]
fn historical_novel_preference_misses_are_regressions_with_unchanged_gold() {
    let corpus: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/phase22/extraction-v3.json")).unwrap();
    for case in corpus["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["category"].as_str().unwrap().contains("novel"))
    {
        let text = case["text"].as_str().unwrap();
        let gold = &case["expected"][0];
        let claims = lucille_memory::dialogue::extract(text, "demo");
        assert!(
            claims
                .iter()
                .any(|c| c.property == gold["property"].as_str().unwrap()
                    && c.value == gold["value"].as_str().unwrap()
                    && lucille_memory::dialogue::safe_preference(c)),
            "{case}"
        );
    }
}

#[test]
fn concurrent_vector_writer_and_collection_cannot_resurrect_an_obsolete_version() {
    use std::sync::{Arc, Barrier};
    let (d, mut s) = setup(true);
    learn(&mut s, "Prefiro café sem açúcar.", "preference");
    let space = lucille_memory::semantic::EmbeddingSpace {
        model: "synthetic-concurrent-gc".into(),
        revision: "1".into(),
        fingerprint: "d".repeat(64),
        dimension: 2,
        normalization: "l2".into(),
        pooling: "mean".into(),
        document_prefix: "doc: ".into(),
        query_prefix: "query: ".into(),
        context_tokens: 2048,
        index_version: 5,
    };
    s.configure_embedding_space(&space, &permit()).unwrap();
    let memory = s.list("lucille", "demo").unwrap().pop().unwrap();
    let old = memory.clone();
    let path = d.path().join("synthetic.sqlite3");
    let second_path = path.clone();
    let barrier = Arc::new(Barrier::new(2));
    let second_barrier = barrier.clone();
    let worker = std::thread::spawn(move || {
        let mut other = Store::open(&second_path).unwrap();
        second_barrier.wait();
        for _ in 0..20 {
            let _ =
                other.put_memory_vector(&old.input.id, old.version, &space, &[1., 0.], &permit());
            other.prune_obsolete_vectors().unwrap();
        }
    });
    barrier.wait();
    learn(
        &mut s,
        "Na verdade, agora prefiro café com açúcar.",
        "preference",
    );
    worker.join().unwrap();
    assert_eq!(s.prune_obsolete_vectors().unwrap(), 0);
    let db = rusqlite::Connection::open(path).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM memory_vectors WHERE memory_id=?1",
            [&memory.input.id],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(s.memory_details("lucille", "demo").unwrap().len(), 2);
    assert!(!s.history(&memory.input.id).unwrap().is_empty());
}

#[test]
fn temporal_tracks_and_autonomous_history_roundtrip_backups_and_revision_restore() {
    let (_d, mut s) = setup(true);
    for (text, category) in [
        ("Meu projeto usa Rust.", "fact"),
        ("Quero usar Python.", "decision"),
    ] {
        learn(&mut s, text, category);
        let m = s
            .list("lucille", "demo")
            .unwrap()
            .into_iter()
            .find(|m| m.state == Status::Candidate)
            .unwrap();
        s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
            .unwrap();
    }
    learn(&mut s, "Prefiro café sem açúcar.", "preference");
    learn(
        &mut s,
        "Na verdade, agora prefiro café com açúcar.",
        "preference",
    );
    let future = s
        .memory_details("lucille", "demo")
        .unwrap()
        .into_iter()
        .find(|d| d.quality.as_ref().is_some_and(|q| q.temporal == "planned"))
        .unwrap()
        .memory;
    s.change(
        &future.input.id,
        "lucille",
        "demo",
        future.version,
        Action::Obsolete,
    )
    .unwrap();
    s.undo_consolidation(
        &future.input.id,
        "lucille",
        "demo",
        future.version + 1,
        future.version,
        &permit(),
    )
    .unwrap();
    let backup = s.export_backup(&permit()).unwrap();
    let (_other, mut target) = setup(true);
    target.restore_isolated(&backup, &permit()).unwrap();
    assert_eq!(
        serde_json::to_value(s.memory_details("lucille", "demo").unwrap()).unwrap(),
        serde_json::to_value(target.memory_details("lucille", "demo").unwrap()).unwrap()
    );
    for detail in s.memory_details("lucille", "demo").unwrap() {
        let restored = target.history(&detail.memory.input.id).unwrap();
        for change in s.history(&detail.memory.input.id).unwrap() {
            assert!(restored
                .iter()
                .any(|other| other.operation == change.operation
                    && other.version == change.version
                    && other.at == change.at));
        }
        assert!(restored.iter().any(|c| c.operation == "restore"));
    }
}
#[test]
fn same_source_conflicting_preferences_never_auto_promote_and_ids_do_not_collide() {
    let (_d, mut s) = setup(true);
    let text = "Prefiro café sem açúcar; prefiro café com açúcar";
    s.enqueue_learning(input(text, "contradictory")).unwrap();
    let lease = s.claim_llm_learning(now() + 1).unwrap().unwrap();
    let raw=serde_json::json!({"version":3,"scope":"lucille","project":"demo","claims":[{"evidence":"Prefiro café sem açúcar","category":"preference"},{"evidence":"prefiro café com açúcar","category":"preference"}]}).to_string();
    s.complete_llm_learning(&lease, &raw, now() + 2).unwrap();
    let details = s.memory_details("lucille", "demo").unwrap();
    assert_eq!(details.len(), 2);
    assert!(details.iter().all(|d| d.memory.state != Status::Active));
    assert!(s
        .context(request("Como eu prefiro meu café?"))
        .unwrap()
        .facts
        .is_empty());
}
#[test]
fn same_message_current_and_future_choices_keep_distinct_records_and_keys() {
    let (_d, mut s) = setup(true);
    let text = "Meu projeto usa Rust; Quero usar Python";
    s.enqueue_learning(input(text, "tracks-one-source"))
        .unwrap();
    let lease = s.claim_llm_learning(now() + 1).unwrap().unwrap();
    let raw=serde_json::json!({"version":3,"scope":"lucille","project":"demo","claims":[{"evidence":"Quero usar Python","category":"decision"},{"evidence":"Meu projeto usa Rust","category":"fact"}]}).to_string();
    s.complete_llm_learning(&lease, &raw, now() + 2).unwrap();
    let details = s.memory_details("lucille", "demo").unwrap();
    assert_eq!(details.len(), 2);
    assert_ne!(
        details[0].memory.input.fact_key,
        details[1].memory.input.fact_key
    );
    for d in details {
        let q = d.quality.unwrap();
        assert!(d.memory.input.content.to_lowercase().contains(&q.value));
    }
}

#[test]
fn same_interaction_facet_enrichment_preserves_a_literal_contiguous_source() {
    let (_d, mut s) = setup(true);
    let text = "Prefiro RPGs táticos; Gosto de RPGs táticos com classes e progressão";
    s.enqueue_learning(input(text, "multi-facets")).unwrap();
    let lease = s.claim_llm_learning(now() + 1).unwrap().unwrap();
    let raw=serde_json::json!({"version":3,"scope":"lucille","project":"demo","claims":[{"evidence":"Prefiro RPGs táticos","category":"preference"},{"evidence":"Gosto de RPGs táticos com classes e progressão","category":"preference"}]}).to_string();
    s.complete_llm_learning(&lease, &raw, now() + 2).unwrap();
    let rows = s.memory_details("lucille", "demo").unwrap();
    assert_eq!(rows.len(), 1);
    assert!(
        rows[0]
            .quality
            .as_ref()
            .unwrap()
            .facets
            .iter()
            .any(|f| f == "classes"),
        "claims={:?}; rows={:?}",
        lucille_memory::dialogue::extract(text, "demo"),
        rows
    );
    assert_eq!(rows[0].provenance.len(), 1);
    assert_eq!(rows[0].provenance[0].evidence, text);
}

#[test]
fn preference_authorship_uses_words_not_a_name_suffix_and_preserves_unicode_negation() {
    for text in [
        "Satoshi prefers tactical RPGs with classes.",
        "My favorite actor likes tactical RPGs with classes.",
    ] {
        assert!(
            lucille_memory::dialogue::extract(text, "demo").is_empty(),
            "{text}"
        );
    }
    let (_d, mut s) = setup(true);
    learn(&mut s, "I don’t like tactical RPGs.", "preference");
    let rows = s.memory_details("lucille", "demo").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].quality.as_ref().unwrap().value, "negative");
    assert_eq!(rows[0].provenance[0].evidence, "I don’t like tactical RPGs");
}

#[test]
fn implicit_two_device_query_retains_both_entities_without_contamination() {
    let (_d, mut s) = setup(true);
    for text in [
        "Meu computador principal tem 24 GB de RAM.",
        "Meu notebook de trabalho tem 16 GB de RAM.",
        "Meu outro computador tem 8 GB de RAM.",
    ] {
        learn(&mut s, text, "hardware");
        let m = s
            .list("lucille", "demo")
            .unwrap()
            .into_iter()
            .find(|m| m.state == Status::Candidate)
            .unwrap();
        s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
            .unwrap();
    }
    let ctx = s
        .context(request(
            "Quanto de RAM tem meu principal e meu notebook de trabalho?",
        ))
        .unwrap();
    assert!(ctx
        .facts
        .iter()
        .any(|f| f.entity == "device.primary_computer" && f.value == "24_gb"));
    assert!(ctx
        .facts
        .iter()
        .any(|f| f.entity == "device.work_laptop" && f.value == "16_gb"));
    assert!(!ctx
        .facts
        .iter()
        .any(|f| f.entity == "device.other_computer"));
    let one = s
        .context(request("Quanto de RAM tem meu notebook de trabalho?"))
        .unwrap();
    assert!(one.facts.iter().all(|f| f.entity == "device.work_laptop"));
}

#[test]
fn current_language_precedence_and_terminal_project_qualifier_survive_reopen() {
    let (d, mut s) = setup(true);
    for (text, kind) in [
        ("Meu projeto usa Rust.", "fact"),
        ("Quero usar Python.", "decision"),
    ] {
        learn(&mut s, text, kind);
        let m = s
            .list("lucille", "demo")
            .unwrap()
            .into_iter()
            .find(|m| m.state == Status::Candidate)
            .unwrap();
        s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
            .unwrap();
    }
    let ctx = s
        .context(request("Que linguagem está vigente no meu projeto agora?"))
        .unwrap();
    assert!(ctx.facts.iter().any(|f| f.value == "rust"));
    assert!(!ctx.facts.iter().any(|f| f.value == "python"));
    learn(&mut s, "Cancelei meu projeto.", "project");
    let m = s
        .list("lucille", "demo")
        .unwrap()
        .into_iter()
        .find(|m| m.state == Status::Candidate)
        .unwrap();
    s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
        .unwrap();
    drop(s);
    let mut s = Store::open(&d.path().join("synthetic.sqlite3")).unwrap();
    let ctx = s
        .context(request("Que linguagem está vigente no meu projeto agora?"))
        .unwrap();
    assert!(ctx
        .facts
        .iter()
        .any(|f| f.property == "lifecycle" && f.modality == "cancelled"));
    assert!(!ctx.facts.iter().any(|f| f.property == "language"));
    let technical = s
        .context(request(
            "Meu projeto está sendo desenvolvido em Rust agora?",
        ))
        .unwrap();
    assert!(technical
        .facts
        .iter()
        .any(|f| f.property == "lifecycle" && f.modality == "cancelled"));
    assert!(s
        .answer_evidence(&request("Como está meu projeto?"), &ctx.references)
        .unwrap()
        .facts
        .iter()
        .any(|f| f.modality == "cancelled"));
}
