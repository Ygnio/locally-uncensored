use lucille_memory::{
    dialogue,
    intelligent::{Interaction, LearningPolicy},
    llm::ExtractionMode,
    source_authority::{clauses, Origin},
    Action, ConfigurationPermit, Mode, Profile, Status, Store, TurnRequest,
};
use serde_json::{json, Value};

const SCOPE: &str = "canonical-qa";
const PROJECT: &str = "jatoba";
const MISSED: [&str; 7] = [
    "ex-001", "ex-006", "ex-007", "ex-013", "ex-015", "ex-017", "ex-018",
];

fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

fn input(id: &str, source: &str) -> Interaction {
    Interaction {
        conversation_id: "canonical-test".into(),
        message_id: id.into(),
        scope: SCOPE.into(),
        project: PROJECT.into(),
        mode: Mode::Chat,
        role: "user".into(),
        user_text: source.into(),
    }
}

fn open(mode: ExtractionMode) -> (tempfile::TempDir, Store) {
    let root = tempfile::tempdir().unwrap();
    let mut store = Store::open(&root.path().join("synthetic.sqlite3")).unwrap();
    let permit = ConfigurationPermit::explicit_settings_action(true).unwrap();
    store
        .configure_identity(Profile::fixture(), 0, &permit)
        .unwrap();
    store
        .set_learning_policy(
            LearningPolicy {
                enabled: true,
                autonomous_preferences: true,
                extraction_mode: mode,
                ..Default::default()
            },
            &permit,
        )
        .unwrap();
    (root, store)
}

fn nomination(source: &str, category: &str) -> String {
    json!({
        "version":3,"scope":SCOPE,"project":PROJECT,
        "claims":[{"evidence":source,"category":category}]
    })
    .to_string()
}

fn complete(store: &mut Store, output: &str) {
    let at = now() + 1;
    if let Some(lease) = store.claim_learning(at).unwrap() {
        assert!(store
            .complete_learning(&lease, &lease.claims, at + 1)
            .unwrap());
    } else if let Some(lease) = store.claim_llm_learning(at).unwrap() {
        assert!(store.complete_llm_learning(&lease, output, at + 1).unwrap());
    }
}

fn modes() -> [ExtractionMode; 3] {
    [
        ExtractionMode::Rules,
        ExtractionMode::Llm,
        ExtractionMode::Hybrid,
    ]
}

fn request(query: &str) -> TurnRequest {
    TurnRequest {
        conversation_id: "canonical-projection-test".into(),
        message_id: uuid::Uuid::new_v4().to_string(),
        scope: SCOPE.into(),
        project: PROJECT.into(),
        mode: Mode::Agent,
        query: query.into(),
        token_budget: 8192,
    }
}

fn learn_reviewed(store: &mut Store, id: &str, source: &str) {
    let job = store.enqueue_learning(input(id, source)).unwrap();
    assert_eq!(job["state"], "queued", "{source}");
    complete(store, "");
    let candidates: Vec<_> = store
        .memory_details(SCOPE, PROJECT)
        .unwrap()
        .into_iter()
        .filter(|d| d.memory.state == Status::Candidate)
        .collect();
    assert!(!candidates.is_empty(), "{source}");
    for d in candidates {
        store
            .change(
                &d.memory.input.id,
                SCOPE,
                PROJECT,
                d.memory.version,
                Action::Approve,
            )
            .unwrap();
    }
}

#[test]
fn purpose_query_recovers_the_canonical_project_description_without_a_vector() {
    let (_root, mut store) = open(ExtractionMode::Rules);
    learn_reviewed(
        &mut store,
        "purpose",
        "Estou começando um projeto de catálogo de miniaturas.",
    );
    for query in [
        "Qual é a finalidade do projeto de catálogo que comecei?",
        "Qual é o propósito deste projeto?",
    ] {
        let context = store.context(request(query)).unwrap();
        assert!(
            context.facts.iter().any(|f| {
                f.entity == "project.jatoba"
                    && f.property == "description"
                    && f.modality == "project_description"
                    && f.evidence
                        .iter()
                        .any(|e| e.text.contains("catálogo de miniaturas"))
            }),
            "{query}"
        );
    }
    for query in [
        "Qual é a finalidade de usar PostgreSQL no meu projeto?",
        "Qual é a finalidade do projeto Pitanga?",
    ] {
        assert!(
            !store
                .context(request(query))
                .unwrap()
                .facts
                .iter()
                .any(|f| f.property == "description"),
            "{query}"
        );
    }
}

#[test]
fn cancellation_qualifies_a_prior_plan_without_transferring_it_to_the_brother() {
    let (_root, mut store) = open(ExtractionMode::Rules);
    learn_reviewed(
        &mut store,
        "database-plan",
        "Pretendo usar PostgreSQL como banco de dados no meu projeto.",
    );
    let query = "Compare o banco planejado no meu projeto com o banco que meu irmão usa.";
    let context = store.context(request(query)).unwrap();
    let plan = context
        .facts
        .iter()
        .find(|f| f.property == "database")
        .unwrap()
        .clone();
    assert_eq!((&*plan.modality, &*plan.validity), ("planned", "current"));
    learn_reviewed(
        &mut store,
        "cancellation",
        "Eu cancelei meu projeto de catálogo.",
    );
    let context = store.context(request(query)).unwrap();
    assert!(context.facts.iter().any(|f| {
        f.id == plan.id
            && f.version == plan.version
            && f.value == "postgresql"
            && f.modality == "planned"
            && f.validity == "historical"
    }));
    assert!(context.facts.iter().any(|f| {
        f.entity == "project.jatoba"
            && f.property == "lifecycle"
            && f.modality == "cancelled"
            && f.validity == "current"
    }));
    assert!(context
        .relations
        .iter()
        .any(|r| r.actor == "third_party" && r.references.is_empty()));
    assert!(!store
        .context(request("Que banco o meu projeto usa atualmente?"))
        .unwrap()
        .facts
        .iter()
        .any(|f| f.property == "database" && f.validity == "current"));
}

#[test]
fn frozen_seven_relations_materialize_with_original_evidence_in_every_mode() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/phase22e/final-v1/extraction.json")).unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    for id in MISSED {
        let case = cases.iter().find(|case| case["id"] == id).unwrap();
        let text = case["text"].as_str().unwrap();
        let expected = &case["expected"][0];
        let category = expected["kind"].as_str().unwrap();
        for mode in modes() {
            let (_root, mut store) = open(mode.clone());
            let job = store.enqueue_learning(input(id, text)).unwrap();
            assert_eq!(job["state"], "queued", "{id} {mode:?}: {job}");
            complete(&mut store, &nomination(text, category));
            let details = store.memory_details(SCOPE, PROJECT).unwrap();
            assert_eq!(details.len(), 1, "{id} {mode:?}: {details:?}");
            let detail = &details[0];
            let q = detail.quality.as_ref().unwrap();
            assert_eq!(q.subject, expected["subject"], "{id} {mode:?}");
            assert_eq!(q.property, expected["property"], "{id} {mode:?}");
            assert_eq!(q.value, expected["value"], "{id} {mode:?}");
            assert_eq!(q.temporal, expected["temporal"], "{id} {mode:?}");
            assert_eq!(
                detail.memory.state,
                if category == "preference" {
                    Status::Active
                } else {
                    Status::Candidate
                },
                "{id} {mode:?}",
            );
            let claim = dialogue::extract(text, PROJECT).pop().unwrap();
            let proof = claim.authority.as_ref().unwrap();
            assert_eq!(proof.origin, Origin::PersonalDeclaration, "{id}");
            assert_eq!(&text[proof.start..proof.end], claim.evidence, "{id}");
            assert_eq!(
                detail.provenance[0].evidence, claim.evidence,
                "{id} {mode:?}"
            );
        }
    }
}

#[test]
fn relation_paraphrases_preserve_value_modality_and_device() {
    for (text, subject, property, value, temporal) in [
        (
            "Coffee without sugar is my preference in the afternoon.",
            "user",
            "coffee_sugar",
            "without_sugar",
            "preference",
        ),
        (
            "A minha preferência para o café é café com açúcar.",
            "user",
            "coffee_sugar",
            "with_sugar",
            "preference",
        ),
        (
            "O tema escuro é a minha preferência para a interface.",
            "user",
            "interface_theme",
            "dark",
            "preference",
        ),
        (
            "My preferred response language is Portuguese.",
            "user",
            "response_language",
            "pt",
            "preference",
        ),
        (
            "Turn-based strategy games are among my preferences.",
            "user",
            "games_turn_strategy",
            "positive",
            "preference",
        ),
        (
            "My database choice for this project is SQLite.",
            "project.jatoba",
            "database",
            "sqlite",
            "durable",
        ),
        (
            "Pretendo usar SQLite como banco de dados no projeto.",
            "project.jatoba",
            "database",
            "sqlite",
            "planned",
        ),
        (
            "No computador principal que uso há 48 GB de RAM.",
            "device.primary_computer",
            "ram_capacity",
            "48_gb",
            "durable",
        ),
        (
            "Na máquina de trabalho, meu notebook de trabalho conta com 24 GB de RAM.",
            "device.work_laptop",
            "ram_capacity",
            "24_gb",
            "durable",
        ),
        (
            "On the primary computer I use, 24 GB of RAM are installed.",
            "device.primary_computer",
            "ram_capacity",
            "24_gb",
            "durable",
        ),
        (
            "No meu outro computador estão instalados 48 GB de RAM.",
            "device.other_computer",
            "ram_capacity",
            "48_gb",
            "durable",
        ),
    ] {
        let claims = dialogue::extract(text, PROJECT);
        assert_eq!(claims.len(), 1, "{text}: {claims:?}");
        let c = &claims[0];
        assert_eq!(
            (
                c.subject.as_str(),
                c.property.as_str(),
                c.value.as_str(),
                c.temporal.as_str()
            ),
            (subject, property, value, temporal),
            "{text}",
        );
    }
}

#[test]
fn ownership_does_not_materialize_unproved_or_nonasserted_values() {
    for text in [
        "Segundo meu irmão, o computador tem 48 GB.",
        "Não sei se meu computador possui 48 GB.",
        "O manual recomenda 48 GB.",
        "A personagem diz: meu computador tem 48 GB.",
        "Gostaria de instalar 48 GB.",
        "Meu computador não possui 48 GB.",
        "O meu computador principal conta que eu prefiro café sem açúcar.",
        "No computador principal que uso não há 48 GB de RAM.",
        "No computador principal que uso talvez estejam instalados 48 GB de RAM.",
        "No computador principal que uso há 48 GB de RAM livres.",
        "No computador principal que uso há 48 GB de VRAM.",
        "On the primary computer I use, 24 GB or 48 GB of RAM are installed.",
        "A minha preferência seria café sem açúcar se houvesse.",
        "Café sem açúcar ou café com açúcar é a minha preferência.",
        "Café sem açúcar é o contrário da minha preferência.",
        "My preference has changed: I now prefer my brother to drink coffee without sugar.",
        "O meu notebook de trabalho conta com previsão de 48 GB de RAM.",
        "Meu irmão conta que café sem açúcar é a minha preferência.",
        "For project Pitanga, PostgreSQL was my database choice.",
    ] {
        assert!(
            dialogue::extract(text, PROJECT).iter().all(|c| ![
                "ram_capacity",
                "coffee_sugar",
                "database",
            ]
            .contains(&c.property.as_str())),
            "{text}",
        );
    }
    let own = "No computador principal que uso, a capacidade dependeria da configuração.";
    assert_eq!(clauses(own)[0].origin, Origin::PersonalDeclaration);
    assert!(dialogue::extract(own, PROJECT).is_empty());
    assert_eq!(
        clauses("Segundo meu irmão, o computador tem 48 GB.")[0].origin,
        Origin::ReportedSpeech,
    );
}

#[test]
fn new_nominal_parses_keep_quote_and_tool_authority_separate() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/phase22d/final-v1/extraction.json")).unwrap();
    for id in ["dx-031", "dx-032"] {
        let case = fixture["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == id)
            .unwrap();
        let text = case["text"].as_str().unwrap();
        for mode in modes() {
            let (_root, mut store) = open(mode);
            store.enqueue_learning(input(id, text)).unwrap();
            complete(&mut store, &nomination(text, "preference"));
            assert!(
                store.memory_details(SCOPE, PROJECT).unwrap().is_empty(),
                "{text}"
            );
        }
    }
    for text in [
        "Tool output\nMy preferred interface appearance is the light theme.",
        "Trecho do documento\nCafé sem açúcar é a minha preferência.",
        "A personagem diz: no computador principal que uso há 48 GB de RAM.",
        "O manual explica que no computador principal que uso há 48 GB de RAM.",
    ] {
        assert!(dialogue::extract(text, PROJECT).is_empty(), "{text}");
    }
    let claims = dialogue::extract("Eu prefiro café, mas o narrador prefere chá.", PROJECT);
    assert_eq!(claims.len(), 1);
    assert_eq!(
        (&*claims[0].property, &*claims[0].value),
        ("beverage_preference", "coffee")
    );
    assert!(!claims[0].evidence.contains("chá"));
}

#[test]
fn correction_committed_first_cannot_be_replaced_by_a_slower_older_source() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/phase22e/final-v1/faults.json")).unwrap();
    let case = fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["id"] == "final-e-source-order")
        .unwrap();
    let older = case["texts"][0].as_str().unwrap();
    let newer = case["texts"][1].as_str().unwrap();
    for mode in modes() {
        let (_root, mut store) = open(mode.clone());
        let first = store.begin_user_turn(input("older", older)).unwrap();
        let distinct = "No computador principal que uso há 48 GB de RAM.";
        let separate = store.begin_user_turn(input("separate", distinct)).unwrap();
        let second = store.begin_user_turn(input("newer", newer)).unwrap();
        let first = serde_json::from_value(first["proof"].clone()).unwrap();
        let separate = serde_json::from_value(separate["proof"].clone()).unwrap();
        let second = serde_json::from_value(second["proof"].clone()).unwrap();
        store.publish_user_turn(second).unwrap();
        complete(&mut store, &nomination(newer, "preference"));
        let before = store.memory_details(SCOPE, PROJECT).unwrap();
        assert_eq!(before.len(), 1, "{mode:?}");
        assert_eq!(before[0].quality.as_ref().unwrap().value, "without_sugar");
        assert_eq!(before[0].memory.state, Status::Active);
        store.publish_user_turn(first).unwrap();
        complete(&mut store, &nomination(older, "preference"));
        let after = store.memory_details(SCOPE, PROJECT).unwrap();
        assert_eq!(
            serde_json::to_value(&after).unwrap(),
            serde_json::to_value(&before).unwrap()
        );
        store.publish_user_turn(separate).unwrap();
        complete(&mut store, &nomination(distinct, "hardware"));
        let separate_after = store.memory_details(SCOPE, PROJECT).unwrap();
        assert_eq!(separate_after.len(), 2, "{mode:?}");
        assert!(separate_after.iter().any(|detail| {
            detail.memory.state == Status::Candidate
                && detail
                    .quality
                    .as_ref()
                    .is_some_and(|q| q.subject == "device.primary_computer" && q.value == "48_gb")
        }));
        assert!(store
            .learning_jobs(SCOPE, PROJECT)
            .unwrap()
            .iter()
            .all(|j| j.state == "done"));
    }
}

#[test]
fn later_explicit_correction_preserves_superseded_preference_in_history() {
    let before = "For my afternoon coffee, I prefer it with sugar.";
    let after = "My preference has changed: for my own coffee, I now prefer it without sugar.";
    for mode in modes() {
        let (_root, mut store) = open(mode.clone());
        store.enqueue_learning(input("before", before)).unwrap();
        complete(&mut store, &nomination(before, "preference"));
        store.enqueue_learning(input("after", after)).unwrap();
        complete(&mut store, &nomination(after, "preference"));
        let records = store.memory_details(SCOPE, PROJECT).unwrap();
        assert_eq!(records.len(), 2, "{mode:?}");
        assert!(records.iter().any(|d| {
            d.memory.state == Status::Obsolete
                && d.memory.version == 2
                && d.quality.as_ref().unwrap().value == "with_sugar"
                && d.provenance[0].evidence == before.trim_end_matches('.')
        }));
        assert!(records.iter().any(|d| {
            d.memory.state == Status::Active
                && d.memory.version == 1
                && d.quality.as_ref().unwrap().value == "without_sugar"
                && d.provenance[0].evidence == after.trim_end_matches('.')
        }));
    }
}
