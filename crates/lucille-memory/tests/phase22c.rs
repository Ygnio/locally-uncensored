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
fn descriptive_continuation_is_reviewed_not_retried_or_auto_promoted() {
    let (_d, mut s) = setup(false);
    let text = "O projeto continua sendo um catálogo de minerais.";
    learn(&mut s, text, "project");
    let rows = s.memory_details("lucille", "demo").unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].memory.state, Status::Candidate);
    assert_eq!(rows[0].quality.as_ref().unwrap().property, "description");
    s.change(
        &rows[0].memory.input.id,
        "lucille",
        "demo",
        rows[0].memory.version,
        Action::Approve,
    )
    .unwrap();
    learn(&mut s, text, "project");
    assert_eq!(s.memory_details("lucille", "demo").unwrap().len(), 1);
    assert!(s
        .learning_jobs("lucille", "demo")
        .unwrap()
        .iter()
        .all(|j| j.state == "done"));
}
#[test]
fn temporal_connectives_are_not_foreign_names_but_explicit_names_still_are() {
    for q in [
        "Antes do cancelamento, que linguagem eu usava e qual é o status do projeto hoje?",
        "O projeto continua sendo um catálogo de minerais.",
        "Como está meu projeto neste momento?",
    ] {
        assert!(lucille_memory::dialogue::project_scope_matches(q, "demo"));
    }
    assert!(!lucille_memory::dialogue::project_scope_matches(
        "O projeto hoje2 usa Rust.",
        "demo"
    ));
    assert!(!lucille_memory::dialogue::project_scope_matches(
        "O projeto Hoje usa Rust.",
        "demo"
    ));
    assert!(!lucille_memory::dialogue::project_scope_matches(
        "O projeto “hoje” usa Rust.",
        "demo"
    ));
    assert!(!lucille_memory::dialogue::project_scope_matches(
        "O projeto \"hoje\" usa Rust.",
        "demo"
    ));
}
#[test]
fn compound_history_retrieves_the_previous_language_and_current_cancellation() {
    let (_d, mut s) = setup(true);
    for (text, kind) in [
        ("Estou implementando meu projeto em TypeScript.", "fact"),
        ("Cancelei meu projeto.", "project"),
    ] {
        learn(&mut s, text, kind);
        let row = s
            .memory_details("lucille", "demo")
            .unwrap()
            .into_iter()
            .find(|d| d.memory.state == Status::Candidate)
            .unwrap_or_else(|| {
                panic!(
                    "No candidate after {text}; rows={:?}; jobs={:?}",
                    s.memory_details("lucille", "demo"),
                    s.learning_jobs("lucille", "demo")
                )
            });
        s.change(
            &row.memory.input.id,
            "lucille",
            "demo",
            row.memory.version,
            Action::Approve,
        )
        .unwrap();
    }
    let c = s
        .context(request(
            "Antes do cancelamento, que linguagem eu usava e qual é o status do projeto hoje?",
        ))
        .unwrap();
    assert!(c
        .facts
        .iter()
        .any(|f| f.property == "language" && f.validity == "historical"));
    assert!(c
        .facts
        .iter()
        .any(|f| f.property == "lifecycle" && f.modality == "cancelled"));
    assert!(c
        .memories
        .iter()
        .all(|m| m.input.scope == "lucille" && m.input.project == "demo"));
    assert!(s
        .context(request("Como está meu projeto neste momento?"))
        .unwrap()
        .facts
        .iter()
        .any(|f| f.property == "lifecycle" && f.modality == "cancelled"));
}

#[test]
fn deterministic_foreign_source_rejection_revokes_the_lease_without_three_model_attempts() {
    let (_d, mut s) = setup(true);
    let text = "O projeto Boreal usa Rust.";
    s.enqueue_learning(input(text, "foreign")).unwrap();
    let lease = s.claim_llm_learning(now() + 1).unwrap().unwrap();
    assert!(s
        .complete_llm_learning(&lease, &output(text, "fact"), now() + 2)
        .is_err());
    let jobs = s.learning_jobs("lucille", "demo").unwrap();
    assert_eq!(jobs[0].state, "failed");
    assert_eq!(jobs[0].attempts, 1);
    assert_eq!(jobs[0].error_code.as_deref(), Some("source_ineligible"));
    assert!(s.claim_llm_learning(now() + 100).unwrap().is_none());
    assert!(!s
        .complete_llm_learning(&lease, &output(text, "fact"), now() + 2)
        .unwrap());
    assert!(s.memory_details("lucille", "demo").unwrap().is_empty());
}
