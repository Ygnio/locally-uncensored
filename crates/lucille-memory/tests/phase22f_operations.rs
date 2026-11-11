use lucille_memory::{
    intelligent::{Interaction, LearningPolicy},
    turn_sources::SourceProof,
    Action, ConfigurationPermit, Kind, Mode, Profile, Store, TurnRequest,
};
use serde_json::Value;
use std::{path::Path, time::SystemTime};

fn now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn permit() -> ConfigurationPermit {
    ConfigurationPermit::explicit_settings_action(true).unwrap()
}

fn setup(path: &Path, auto: bool) -> Store {
    let mut store = Store::open(path).unwrap();
    store
        .configure_identity(Profile::fixture(), 0, &permit())
        .unwrap();
    store
        .set_learning_policy(
            LearningPolicy {
                enabled: true,
                autonomous_preferences: auto,
                auto_kinds: if auto { vec![Kind::Preference] } else { vec![] },
                ..Default::default()
            },
            &permit(),
        )
        .unwrap();
    store
}

fn prepare(store: &mut Store, message: &str, text: &str, mode: Mode) -> (TurnRequest, SourceProof) {
    let request = TurnRequest {
        conversation_id: "operation-chat".into(),
        message_id: format!("answer-{message}"),
        scope: "qa".into(),
        project: "pitanga".into(),
        mode: mode.clone(),
        query: text.into(),
        token_budget: 8192,
    };
    let begun = store
        .begin_user_turn(Interaction {
            conversation_id: request.conversation_id.clone(),
            message_id: message.into(),
            scope: request.scope.clone(),
            project: request.project.clone(),
            mode,
            role: "user".into(),
            user_text: text.into(),
        })
        .unwrap();
    (
        request,
        serde_json::from_value(begun["proof"].clone()).unwrap(),
    )
}

fn learning(store: &mut Store, request: &TurnRequest, proof: &SourceProof) -> Option<Value> {
    store
        .answer_evidence_for_source(request, &[], &[], Some(proof))
        .unwrap()
        .learning_receipt
}

#[test]
fn no_receipt_before_transaction_and_rollback_never_grants_authority() {
    for mode in [Mode::Chat, Mode::Agent, Mode::Code] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("synthetic.sqlite3");
        let mut store = setup(&path, true);
        let (request, proof) = prepare(&mut store, "first", "Eu prefiro café sem açúcar.", mode);
        let evidence = store
            .answer_evidence_for_source(&request, &[], &[], Some(&proof))
            .unwrap();
        assert_eq!(
            evidence.source_operation.unwrap()["state"],
            "source_persisted"
        );
        assert!(evidence.learning_receipt.is_none());
        store.publish_user_turn(proof.clone()).unwrap();
        let lease = store.claim_learning(now() + 1).unwrap().unwrap();
        let evidence = store
            .answer_evidence_for_source(&request, &[], &[], Some(&proof))
            .unwrap();
        assert_eq!(evidence.source_operation.unwrap()["state"], "processing");
        assert!(evidence.learning_receipt.is_none());
        let db = rusqlite::Connection::open(&path).unwrap();
        db.execute_batch("CREATE TRIGGER synthetic_receipt_failure BEFORE INSERT ON memory_source_operation_records BEGIN SELECT RAISE(ABORT,'synthetic receipt write failure'); END;").unwrap();
        assert!(store
            .complete_learning(&lease, &lease.claims, now() + 2)
            .is_err());
        assert!(store.list("qa", "pitanga").unwrap().is_empty());
        assert!(learning(&mut store, &request, &proof).is_none());
        db.execute_batch("DROP TRIGGER synthetic_receipt_failure")
            .unwrap();
        assert!(store
            .complete_learning(&lease, &lease.claims, now() + 2)
            .unwrap());
        let receipt = learning(&mut store, &request, &proof).unwrap();
        assert_eq!(receipt["identity_version"], 1);
        assert_eq!(receipt["records"][0]["effect"], "create");
        assert_eq!(receipt["records"][0]["state"], "active");
        assert_eq!(receipt["records"][0]["version"], 1);
        assert_eq!(receipt["source_message_id"], "first");
        let (same, other) = prepare(
            &mut store,
            "duplicate",
            "Eu prefiro café sem açúcar.",
            request.mode.clone(),
        );
        store.publish_user_turn(other.clone()).unwrap();
        let second = store.claim_learning(now() + 3).unwrap().unwrap();
        store
            .complete_learning(&second, &second.claims, now() + 4)
            .unwrap();
        assert!(
            learning(&mut store, &same, &other).is_none(),
            "new provenance alone is not a record creation"
        );
        drop(store);
        let mut store = Store::open(&path).unwrap();
        assert_eq!(learning(&mut store, &request, &proof).unwrap(), receipt);
        store
            .configure_identity(Profile::fixture(), 1, &permit())
            .unwrap();
        assert!(learning(&mut store, &request, &proof).is_none());
    }
}

#[test]
fn candidate_and_review_are_distinct_versions_and_backup_checks_completed_proof() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = setup(&dir.path().join("source.sqlite3"), false);
    let (request, proof) = prepare(
        &mut store,
        "candidate",
        "Eu prefiro café sem açúcar.",
        Mode::Agent,
    );
    store.publish_user_turn(proof.clone()).unwrap();
    let lease = store.claim_learning(now() + 1).unwrap().unwrap();
    store
        .complete_learning(&lease, &lease.claims, now() + 2)
        .unwrap();
    let receipt = learning(&mut store, &request, &proof).unwrap();
    assert_eq!(receipt["records"][0]["state"], "candidate");
    let backup = store.export_backup(&permit()).unwrap();
    let mut restored = Store::open(&dir.path().join("restored.sqlite3")).unwrap();
    restored.restore_isolated(&backup, &permit()).unwrap();
    assert_eq!(
        learning(&mut restored, &request, &proof),
        Some(receipt.clone())
    );
    assert_eq!(restored.list("qa", "pitanga").unwrap().len(), 1);
    let mut corrupted: Value = serde_json::from_str(&backup).unwrap();
    corrupted["intelligent"]["tables"]["memory_source_operation_records"][0][2] = 99.into();
    assert!(restored
        .restore_isolated(&corrupted.to_string(), &permit())
        .is_err());
    assert_eq!(
        learning(&mut restored, &request, &proof),
        Some(receipt.clone())
    );
    let mut legacy: Value = serde_json::from_str(&backup).unwrap();
    let tables = legacy["intelligent"]["tables"].as_object_mut().unwrap();
    tables.remove("memory_source_operation_records");
    tables.remove("memory_source_operations");
    restored
        .restore_isolated(&legacy.to_string(), &permit())
        .unwrap();
    assert!(learning(&mut restored, &request, &proof).is_none());
    let id = receipt["records"][0]["id"].as_str().unwrap();
    let changed = store
        .change(id, "qa", "pitanga", 1, Action::Approve)
        .unwrap()
        .unwrap();
    assert_eq!(changed.version, 2);
    assert!(
        learning(&mut store, &request, &proof).is_none(),
        "the original operation cannot attest a different version"
    );
}

#[test]
fn correction_receipt_attests_only_the_new_record_and_its_update_effect() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = setup(&dir.path().join("synthetic.sqlite3"), true);
    let (older, original) = prepare(
        &mut store,
        "older",
        "Eu prefiro café sem açúcar.",
        Mode::Agent,
    );
    store.publish_user_turn(original.clone()).unwrap();
    let first = store.claim_learning(now() + 1).unwrap().unwrap();
    assert!(store
        .complete_learning(&first, &first.claims, now() + 2)
        .unwrap());
    let previous = learning(&mut store, &older, &original).unwrap();
    let (latest, correction) = prepare(
        &mut store,
        "correction",
        "Na verdade, agora prefiro café com açúcar.",
        Mode::Agent,
    );
    store.publish_user_turn(correction.clone()).unwrap();
    let second = store.claim_learning(now() + 3).unwrap().unwrap();
    assert!(store
        .complete_learning(&second, &second.claims, now() + 4)
        .unwrap());
    let updated = learning(&mut store, &latest, &correction).unwrap();
    assert_eq!(updated["records"][0]["effect"], "update");
    assert_eq!(updated["records"][0]["value"], "with_sugar");
    assert_ne!(updated["records"][0]["id"], previous["records"][0]["id"]);
    assert!(learning(&mut store, &older, &original).is_none());
    assert!(store
        .answer_evidence_for_source(&latest, &[], &[], Some(&original))
        .is_err());
}

#[test]
fn forget_receipt_is_bound_to_removed_record_version_after_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic.sqlite3");
    let mut store = setup(&path, true);
    let (learn_request, learn_proof) = prepare(
        &mut store,
        "learn",
        "Eu prefiro café sem açúcar.",
        Mode::Chat,
    );
    store.publish_user_turn(learn_proof.clone()).unwrap();
    let lease = store.claim_learning(now() + 1).unwrap().unwrap();
    store
        .complete_learning(&lease, &lease.claims, now() + 2)
        .unwrap();
    let learned = learning(&mut store, &learn_request, &learn_proof).unwrap();
    let (request, proof) = prepare(
        &mut store,
        "forget",
        "Esqueça minha preferência por café.",
        Mode::Code,
    );
    let receipt = store
        .answer_evidence_for_source(&request, &[], &[], Some(&proof))
        .unwrap()
        .directive_receipt
        .unwrap();
    assert_eq!(receipt["records"][0]["id"], learned["records"][0]["id"]);
    assert_eq!(receipt["records"][0]["version"], 2);
    assert_eq!(receipt["records"][0]["state"], "deleted");
    assert_eq!(receipt["mode"], "code");
    assert!(learning(&mut store, &learn_request, &learn_proof).is_none());
    drop(store);
    let mut store = Store::open(&path).unwrap();
    assert_eq!(
        store
            .answer_evidence_for_source(&request, &[], &[], Some(&proof))
            .unwrap()
            .directive_receipt,
        Some(receipt)
    );
}
