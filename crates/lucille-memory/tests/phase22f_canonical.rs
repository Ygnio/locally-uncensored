use lucille_memory::{
    dialogue,
    intelligent::{Interaction, LearningPolicy},
    llm::ExtractionMode,
    semantic::{EmbeddingSpace, QueryVector},
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
    learn_reviewed_in(store, id, PROJECT, source);
}

fn learn_reviewed_in(store: &mut Store, id: &str, project: &str, source: &str) -> Vec<String> {
    let mut interaction = input(id, source);
    interaction.project = project.into();
    let job = store.enqueue_learning(interaction).unwrap();
    assert_eq!(job["state"], "queued", "{source}");
    complete(store, "");
    let candidates: Vec<_> = store
        .memory_details(SCOPE, project)
        .unwrap()
        .into_iter()
        .filter(|d| d.memory.state == Status::Candidate)
        .collect();
    assert!(!candidates.is_empty(), "{source}");
    candidates
        .into_iter()
        .map(|d| {
            store
                .change(
                    &d.memory.input.id,
                    SCOPE,
                    project,
                    d.memory.version,
                    Action::Approve,
                )
                .unwrap();
            d.memory.input.id
        })
        .collect()
}

#[test]
fn known_project_relations_use_only_the_requested_property_and_track_in_each_ranker() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/phase22e/final-v1/retrieval.json")).unwrap();
    let gui: Value = serde_json::from_str(include_str!(
        "fixtures/phase22e/final-v1/conversation-other.json"
    ))
    .unwrap();
    let seeded = [
        "purpose",
        "language-choice",
        "database-plan",
        "other-database",
        "actual-language",
        "future-language",
    ];
    let (_root, mut store) = open(ExtractionMode::Rules);
    let mut ids = std::collections::HashMap::new();
    for event in fixture["events"].as_array().unwrap() {
        let Some(label) = event["label"]
            .as_str()
            .filter(|label| seeded.contains(label))
        else {
            continue;
        };
        // The older GUI independently demonstrated these two typed sources.
        // This test exercises retrieval regardless of C2 source eligibility.
        let gui_source = match label {
            "actual-language" => Some("go-t02"),
            "future-language" => Some("go-t03"),
            _ => None,
        };
        let source = gui_source
            .and_then(|id| {
                gui["turns"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|turn| turn["id"] == id)
            })
            .unwrap_or(event)["text"]
            .as_str()
            .unwrap();
        let records = learn_reviewed_in(
            &mut store,
            label,
            event["project"].as_str().unwrap(),
            source,
        );
        assert_eq!(records.len(), 1, "{label}");
        ids.insert(label, records[0].clone());
    }
    let space = EmbeddingSpace {
        model: "synthetic-equal-similarity".into(),
        revision: "1".into(),
        fingerprint: "a".repeat(64),
        dimension: 2,
        normalization: "l2".into(),
        pooling: "mean".into(),
        document_prefix: "doc: ".into(),
        query_prefix: "query: ".into(),
        context_tokens: 2048,
        index_version: 5,
    };
    let permit = ConfigurationPermit::explicit_settings_action(true).unwrap();
    store.configure_embedding_space(&space, &permit).unwrap();
    for project in [PROJECT, "pitanga"] {
        for detail in store.memory_details(SCOPE, project).unwrap() {
            let m = detail.memory;
            // Every candidate has the same maximum cosine. Property and
            // modality, not a convenient score difference, must fence it.
            store
                .put_memory_vector(&m.input.id, m.version, &space, &[1., 0.], &permit)
                .unwrap();
        }
    }
    let vector = QueryVector {
        space,
        vector: vec![1., 0.],
    };
    let queries = ["er-014", "er-018", "er-023", "er-024", "er-025", "er-026"];
    for (lexical_weight, query_vector) in [
        (1., None),
        (0., Some(&vector)),
        (0.7, Some(&vector)),
        (0.7, None),
    ] {
        let mut policy = store.learning_policy().unwrap();
        policy.lexical_weight = lexical_weight;
        policy.semantic_weight = 1. - lexical_weight;
        store.set_learning_policy(policy, &permit).unwrap();
        for q in fixture["queries"].as_array().unwrap() {
            let id = q["id"].as_str().unwrap();
            if !queries.contains(&id) {
                continue;
            }
            let mut req = request(q["query"].as_str().unwrap());
            req.project = q["project"].as_str().unwrap().into();
            let context = store.context_hybrid(req, query_vector).unwrap();
            let mut actual = context
                .references
                .iter()
                .map(|r| r.id.clone())
                .collect::<Vec<_>>();
            let mut expected = q["expected"]
                .as_array()
                .unwrap()
                .iter()
                .map(|label| ids[label.as_str().unwrap()].clone())
                .collect::<Vec<_>>();
            actual.sort();
            expected.sort();
            assert_eq!(actual, expected, "{id} lexical_weight={lexical_weight}");
        }
    }
    let different_project = store
        .context(request(
            "No Pitanga, compare a linguagem em implementação com a planejada para o futuro.",
        ))
        .unwrap();
    assert!(different_project.references.is_empty());
}

#[test]
fn a_bound_project_predicate_is_not_mistaken_for_a_foreign_project_name() {
    let (_root, mut store) = open(ExtractionMode::Rules);
    for (id, source, temporal, value) in [
        (
            "actual-c2",
            "Neste projeto estou implementando em TypeScript.",
            "in_progress",
            "typescript",
        ),
        (
            "future-c2",
            "Pretendo usar Rust no meu projeto futuramente.",
            "planned",
            "rust",
        ),
    ] {
        let claims = dialogue::extract(source, "pitanga");
        assert_eq!(claims.len(), 1, "{source}");
        let c = &claims[0];
        assert_eq!(
            (
                c.subject.as_str(),
                c.property.as_str(),
                c.temporal.as_str(),
                c.value.as_str()
            ),
            ("project.pitanga", "language", temporal, value),
            "{source}",
        );
        assert_eq!(
            learn_reviewed_in(&mut store, id, "pitanga", source).len(),
            1
        );
    }
    let details = store.memory_details(SCOPE, "pitanga").unwrap();
    assert_eq!(details.len(), 2);
    assert!(details.iter().all(|d| d.memory.state == Status::Active));
    for source in [
        "Neste projeto Jatobá estou implementando em TypeScript.",
        "Pretendo usar Rust no meu projeto Futuramente.",
        "A personagem diz: neste projeto estou implementando em TypeScript.",
    ] {
        assert!(dialogue::extract(source, "pitanga").is_empty(), "{source}");
        let mut i = input("foreign-project", source);
        i.project = "pitanga".into();
        assert_eq!(
            store.enqueue_learning(i).unwrap()["state"],
            "ignored",
            "{source}"
        );
    }
}

#[test]
fn exact_named_ram_replacement_and_fronted_coffee_survive_native_review() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/phase22e/final-v1/retrieval.json")).unwrap();
    let events = fixture["events"].as_array().unwrap();
    let source = |label: &str| {
        events.iter().find(|event| event["label"] == label).unwrap()["text"]
            .as_str().unwrap()
    };
    let (_root, mut store) = open(ExtractionMode::Rules);
    let before = learn_reviewed_in(&mut store, "ram-before", PROJECT, source("primary-before"));
    let work = learn_reviewed_in(&mut store, "ram-work", PROJECT, source("work-ram"));
    let correction = source("primary-now");
    assert_eq!(store.enqueue_learning(input("ram-replacement", correction)).unwrap()["state"], "queued");
    complete(&mut store, "");
    let details = store.memory_details(SCOPE, PROJECT).unwrap();
    let current = details.iter().find(|d| d.quality.as_ref().is_some_and(|q| q.value == "64_gb")).unwrap();
    let q = current.quality.as_ref().unwrap();
    assert_eq!((q.subject.as_str(), q.property.as_str()), ("device.primary_computer", "ram_capacity"));
    assert_eq!(q.replaces_id.as_ref(), before.first());
    assert_eq!(current.memory.state, Status::Conflict);
    assert_eq!(current.provenance[0].evidence, correction.trim_end_matches('.'));
    store.change(&current.memory.input.id, SCOPE, PROJECT, current.memory.version, Action::Approve)
        .unwrap();
    let details = store.memory_details(SCOPE, PROJECT).unwrap();
    assert!(details.iter().any(|d| d.memory.input.id == before[0] && d.memory.state == Status::Obsolete));
    assert!(details.iter().any(|d| d.memory.input.id == work[0] && d.memory.state == Status::Active));
    let context = store.context(request("Qual era a RAM do computador principal antes e qual é agora?")).unwrap();
    assert!(context.facts.iter().any(|f| f.value == "48_gb" && f.validity == "historical"));
    assert!(context.facts.iter().any(|f| f.value == "64_gb" && f.validity == "current"));
    assert!(!context.facts.iter().any(|f| f.value == "12_gb"));

    let coffee = source("coffee");
    assert_eq!(store.enqueue_learning(input("coffee-fronted", coffee)).unwrap()["state"], "queued");
    complete(&mut store, "");
    assert!(store.memory_details(SCOPE, PROJECT).unwrap().iter().any(|d| {
        d.memory.state == Status::Active
            && d.quality.as_ref().is_some_and(|q| q.subject == "user"
                && q.property == "coffee_sugar" && q.value == "without_sugar")
            && d.provenance[0].evidence == coffee.trim_end_matches('.')
    }));

    for text in [
        "Meu computador principal agora tem 64 GB de RAM, em vez dos 48 GB do meu notebook de trabalho.",
        "Meu computador principal agora não tem 64 GB de RAM, em vez dos 48 GB anteriores.",
        "O narrador diz: meu computador principal agora tem 64 GB de RAM, em vez dos 48 GB anteriores.",
        "Meu café da tarde eu prefiro que meu irmão tome sem açúcar.",
        "A personagem diz: meu café da tarde eu prefiro sem açúcar.",
    ] {
        assert!(dialogue::extract(text, PROJECT).iter().all(|c|
            !["ram_capacity", "coffee_sugar"].contains(&c.property.as_str())), "{text}");
    }
}

#[test]
fn original_gui_hardware_sources_preserve_both_devices_and_scoped_history() {
    fn clause(text: &str) -> &str {
        text.split_once(". Guarde").unwrap().0
    }
    let plan: Value = serde_json::from_str(include_str!("fixtures/phase22e/final-v1/conversation-main.json")).unwrap();
    let turns = plan["turns"].as_array().unwrap();
    let source = |id: &str| {
        turns.iter().find(|t| t["id"] == id).unwrap()["text"].as_str().unwrap()
    };
    let correction = source("gm-t05");
    for mode in modes() {
        let (_root, mut store) = open(mode);
        let mut record_ids = Vec::new();
        for (id, device, amount) in [
            ("gm-t03", "device.primary_computer", "48_gb"),
            ("gm-t04", "device.work_laptop", "12_gb"),
        ] {
            let text = source(id);
            assert_eq!(store.enqueue_learning(input(id, text)).unwrap()["state"], "queued");
            complete(&mut store, &nomination(clause(text), "hardware"));
            let details = store.memory_details(SCOPE, PROJECT).unwrap();
            let record = details.iter().find(|d| d.quality.as_ref().is_some_and(|q|
                q.subject == device && q.property == "ram_capacity" && q.value == amount)).unwrap();
            assert_eq!((&record.memory.state, record.memory.version), (&Status::Candidate, 1));
            assert_eq!(record.provenance[0].evidence, clause(text));
            let record_id = record.memory.input.id.clone();
            store.change(&record_id, SCOPE, PROJECT, 1, Action::Approve).unwrap();
            record_ids.push(record_id);
        }

        let wrong_prior = correction.replace("48 GB", "12 GB");
        assert_eq!(store.enqueue_learning(input("wrong-prior", &wrong_prior)).unwrap()["state"], "queued");
        complete(&mut store, &nomination(clause(&wrong_prior), "hardware"));
        assert_eq!(store.memory_details(SCOPE, PROJECT).unwrap().len(), 2);

        assert_eq!(store.enqueue_learning(input("gm-t05", correction)).unwrap()["state"], "queued");
        complete(&mut store, &nomination(clause(correction), "hardware"));
        let details = store.memory_details(SCOPE, PROJECT).unwrap();
        let current = details.iter().find(|d| d.quality.as_ref().is_some_and(|q| q.value == "64_gb")).unwrap();
        let quality = current.quality.as_ref().unwrap();
        assert_eq!(quality.subject, "device.primary_computer");
        assert_eq!((quality.replaces_id.as_ref(), quality.replaces_version), (record_ids.first(), Some(2)));
        assert_eq!(current.provenance[0].evidence, clause(correction));
        assert_eq!(current.memory.state, Status::Conflict);
        store.change(&current.memory.input.id, SCOPE, PROJECT, 1, Action::Approve).unwrap();
        let details = store.memory_details(SCOPE, PROJECT).unwrap();
        assert_eq!(details.len(), 3);
        assert!(details.iter().any(|d| d.memory.input.id == record_ids[0] && d.memory.state == Status::Obsolete));
        assert!(details.iter().any(|d| d.memory.input.id == record_ids[1] && d.memory.state == Status::Active));
        let context = store.context(request("Qual era a RAM do computador principal antes e qual é agora?")).unwrap();
        assert!(context.facts.iter().any(|f| f.value == "48_gb" && f.validity == "historical"));
        assert!(context.facts.iter().any(|f| f.value == "64_gb" && f.validity == "current"));
        assert!(!context.facts.iter().any(|f| f.value == "12_gb"));
    }
}

#[test]
fn contextual_hardware_grammar_rejects_foreign_negated_and_stale_priors() {
    for text in [
        "Meu computador principal está com 48 GB de RAM livre.",
        "Meu computador principal não está com 48 GB de RAM instalada.",
        "O narrador diz: meu computador principal está com 48 GB de RAM instalada.",
        "No notebook que meu irmão usa para trabalhar, tenho 12 GB de RAM.",
        "No notebook que uso para trabalhar, tenho acesso a 12 GB de RAM.",
        "No notebook que uso para trabalhar, tenho 12 GB de RAM no desktop.",
        "No notebook que uso para trabalhar, tenho 12 GB de RAM do meu computador principal.",
        "Corrigindo o dado do computador principal: agora ele não possui 64 GB de RAM, não mais os 48 GB.",
        "Corrigindo o dado do computador principal: agora ele possui acesso remoto a 64 GB de RAM, não mais os 48 GB.",
        "Corrigindo o dado do computador principal: agora ele possui 64 GB de RAM, não mais os 48 GB do notebook de trabalho.",
        "O narrador diz: corrigindo o dado do computador principal: agora ele possui 64 GB de RAM, não mais os 48 GB.",
    ] {
        assert!(dialogue::extract(text, PROJECT).iter().all(|c| c.property != "ram_capacity"), "{text}");
    }
    let (_root, mut store) = open(ExtractionMode::Rules);
    let old = "Meu computador principal está com 48 GB de RAM instalada.";
    let correction = "Corrigindo o dado do computador principal: agora ele possui 64 GB de RAM, não mais os 48 GB.";
    let foreign = learn_reviewed_in(&mut store, "foreign-old", "another-project", old);
    assert_eq!(foreign.len(), 1);
    assert_eq!(store.enqueue_learning(input("foreign-prior", correction)).unwrap()["state"], "queued");
    complete(&mut store, "");
    assert!(store.memory_details(SCOPE, PROJECT).unwrap().is_empty());

    let local = learn_reviewed_in(&mut store, "local-old", PROJECT, old);
    assert_eq!(store.enqueue_learning(input("local-prior", correction)).unwrap()["state"], "queued");
    complete(&mut store, "");
    let details = store.memory_details(SCOPE, PROJECT).unwrap();
    let current = details.iter().find(|d| d.memory.state == Status::Conflict).unwrap();
    store.change(&local[0], SCOPE, PROJECT, 2, Action::Delete).unwrap();
    assert_eq!(store.change(&current.memory.input.id, SCOPE, PROJECT, 1, Action::Approve).unwrap_err(),
               "Replacement source changed; review again");
    assert!(store.memory_details(SCOPE, PROJECT).unwrap().iter().all(|d| d.memory.state != Status::Active));
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
