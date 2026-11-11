use lucille_memory::{
    intelligent::{Interaction, LearningPolicy},
    llm::ExtractionMode,
    turn_sources::SourceProof,
    Action, ConfigurationPermit, Mode, Profile, Status, Store, TurnRequest,
};

fn source(text: &str) -> Interaction {
    Interaction {
        conversation_id: "phase22f-forget-development".into(),
        message_id: uuid::Uuid::new_v4().to_string(),
        scope: "synthetic".into(),
        project: "jatoba".into(),
        mode: Mode::Chat,
        role: "user".into(),
        user_text: text.into(),
    }
}

fn request(input: &Interaction) -> TurnRequest {
    TurnRequest {
        conversation_id: input.conversation_id.clone(),
        message_id: input.message_id.clone(),
        scope: input.scope.clone(),
        project: input.project.clone(),
        mode: Mode::Chat,
        query: input.user_text.clone(),
        token_budget: 8192,
    }
}

fn fixture() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("synthetic.sqlite3")).unwrap();
    let permit = ConfigurationPermit::explicit_settings_action(true).unwrap();
    store.configure_identity(Profile::fixture(), 0, &permit).unwrap();
    store.set_learning_policy(
        LearningPolicy {
            enabled: true,
            autonomous_preferences: true,
            extraction_mode: ExtractionMode::Rules,
            ..Default::default()
        },
        &permit,
    ).unwrap();
    (directory, store)
}

fn publish(store: &mut Store, input: &Interaction) -> Option<String> {
    let enqueued = store.enqueue_learning(input.clone()).unwrap();
    let at = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64;
    if let Some(lease) = store.claim_learning(at + 1).unwrap() {
        assert!(store.complete_learning(&lease, &lease.claims, at + 2).unwrap());
    }
    enqueued["id"].as_str().map(str::to_owned)
}

#[test]
fn explicit_emphasis_preserves_delete_replay_and_review_across_restarts() {
    for (initial, directive, renewed, property, probe) in [
        (
            "Eu gosto de jogos em que escolho a classe.",
            "Esqueça de verdade minha preferência pela escolha de classe nos jogos.",
            "Volte a guardar esta preferência: eu gosto de jogos em que escolho a classe.",
            "games_class_selection",
            "De que eu gosto em jogos com classes?",
        ),
        (
            "Eu prefiro café sem açúcar.",
            "Esqueça de verdade como eu prefiro o açúcar no café.",
            "Volte a guardar esta preferência: eu prefiro café sem açúcar.",
            "coffee_sugar",
            "Como prefiro o café em relação ao açúcar?",
        ),
    ] {
        let (directory, mut store) = fixture();
        let db = directory.path().join("synthetic.sqlite3");
        let original = source(initial);
        let _ = publish(&mut store, &original);
        let old = store.list("synthetic", "jatoba").unwrap().pop().unwrap();
        assert_eq!(old.state, Status::Active);
        let forget = source(directive);
        let done = store.enqueue_learning(forget.clone()).unwrap();
        assert_eq!(done["deleted"], 1, "{directive}");
        let receipt = store.answer_evidence_for_source(
            &request(&forget), &[], &[], Some(&SourceProof {
                interaction: forget.clone(),
                job_id: Some(done["id"].as_str().unwrap().into()),
            }),
        ).unwrap().directive_receipt.unwrap();
        assert_eq!(receipt["had_records"], true);
        assert_eq!(receipt["property"], property);
        assert_eq!(store.deleted_memories("synthetic", "jatoba").unwrap().len(), 1);
        drop(store);

        let mut store = Store::open(&db).unwrap();
        assert!(store.context(request(&source(probe))).unwrap().facts.is_empty());
        let _ = publish(&mut store, &original);
        let _ = publish(&mut store, &source(initial));
        assert!(store.list("synthetic", "jatoba").unwrap().is_empty());
        assert_eq!(store.deleted_memories("synthetic", "jatoba").unwrap().len(), 1);

        let fresh = source(renewed);
        let fresh_job = publish(&mut store, &fresh).unwrap();
        let detail = store.memory_details("synthetic", "jatoba").unwrap().pop().unwrap();
        let candidate = detail.memory;
        assert_eq!(candidate.state, Status::Candidate);
        assert_eq!(candidate.version, 1);
        assert_ne!(candidate.input.id, old.input.id);
        assert!(detail.provenance.iter().any(|entry| {
            entry.interaction_id == fresh_job && !entry.evidence.is_empty() && renewed.contains(&entry.evidence)
        }));
        store.change(&candidate.input.id, "synthetic", "jatoba", 1, Action::Approve).unwrap();
        drop(store);

        let mut store = Store::open(&db).unwrap();
        let rows = store.list("synthetic", "jatoba").unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].input.id, candidate.input.id);
        assert_eq!(rows[0].state, Status::Active);
        assert_eq!(rows[0].version, 2);
        assert_eq!(store.deleted_memories("synthetic", "jatoba").unwrap().len(), 1);
        let facts = store.context(request(&source(probe))).unwrap().facts;
        assert_eq!(facts.len(), 1, "{probe}");
        assert_eq!(facts[0].property, property);
        store.check_integrity().unwrap();
    }
}

#[test]
fn mention_negation_question_condition_and_quotes_do_not_delete() {
    let (_directory, mut store) = fixture();
    let _ = publish(&mut store, &source("Eu prefiro café sem açúcar."));
    let original = store.list("synthetic", "jatoba").unwrap();
    assert_eq!(original.len(), 1);
    for text in [
        "A personagem disse: esqueça de verdade como eu prefiro o açúcar no café.",
        "Não esqueça de verdade como eu prefiro o açúcar no café.",
        "Esqueça de verdade como eu prefiro o açúcar no café?",
        "Esqueça de verdade como eu prefiro o açúcar no café se eu pedir.",
        "Por favor, “esqueça de verdade como eu prefiro o açúcar no café”.",
        "Please, ‘forget my coffee preference’.",
    ] {
        let input = source(text);
        let result = store.enqueue_learning(input.clone()).unwrap();
        let receipt = store.answer_evidence_for_source(
            &request(&input), &[], &[], Some(&SourceProof {
                interaction: input,
                job_id: result["id"].as_str().map(str::to_owned),
            }),
        ).unwrap().directive_receipt;
        assert!(receipt.is_none(), "{text}");
        assert!(store.deleted_memories("synthetic", "jatoba").unwrap().is_empty(), "{text}");
        let rows = store.list("synthetic", "jatoba").unwrap();
        assert_eq!(rows.len(), 1, "{text}");
        assert_eq!(rows[0].input.id, original[0].input.id, "{text}");
    }
}
