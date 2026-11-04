use lucille_memory::{dialogue, Kind};

#[test]
fn original_eight_gemma_evidences_have_independently_grounded_relations() {
    for (source, property, value, temporal) in [
        (
            "My own preference is coffee without sugar",
            "coffee_sugar",
            "without_sugar",
            "preference",
        ),
        (
            "Entre as bebidas, minha favorita é café",
            "favorite_beverage",
            "coffee",
            "preference",
        ),
        (
            "My favorite beverage for myself is tea",
            "favorite_beverage",
            "tea",
            "preference",
        ),
        (
            "Para o meu projeto, a linguagem que decidi usar é Rust",
            "language",
            "rust",
            "durable",
        ),
        (
            "My choice for my project database is SQLite",
            "database",
            "sqlite",
            "durable",
        ),
        (
            "Meu computador principal está equipado com 36 GB de RAM",
            "ram_capacity",
            "36_gb",
            "durable",
        ),
        (
            "Tenho como objetivo aprender italiano",
            "goal.",
            "tenho como objetivo aprender italiano",
            "durable",
        ),
        (
            "Já concluí meu objetivo de aprender francês",
            "goal.",
            "completed:aprender frances",
            "completed",
        ),
    ] {
        let raw = serde_json::json!({"version":3,"scope":"test","project":"demo","claims":[{"evidence":source,"category": if property.starts_with("goal.") {"goal"} else if property == "ram_capacity" {"hardware"} else if ["language","database"].contains(&property) {"decision"} else {"preference"}}]}).to_string();
        let claims = dialogue::normalize(source, "test", "demo", &raw)
            .unwrap_or_else(|err| panic!("{source}: {err}"));
        assert_eq!(claims.len(), 1, "{source}: {claims:?}");
        let c = &claims[0];
        assert!(c.property.starts_with(property), "{source}");
        assert_eq!(c.value, value, "{source}");
        assert_eq!(c.temporal, temporal, "{source}");
        assert_eq!(c.evidence, source);
        if c.kind == Kind::Preference {
            assert!(dialogue::safe_preference(c), "{source}");
        }
    }
}

#[test]
fn negation_uncertainty_and_foreign_possession_do_not_create_current_ram_candidates() {
    for source in [
        "Não estou dizendo que meu computador tem 128 GB de RAM",
        "Meu computador não tem 36 GB de RAM",
        "Talvez eu instale 36 GB",
        "Meu irmão possui 36 GB",
        "O manual recomenda 36 GB",
        "Não sei se meu computador tem 36 GB",
        "Quero comprar um computador com 36 GB",
    ] {
        assert!(
            dialogue::extract(source, "demo")
                .iter()
                .all(|c| c.property != "ram_capacity"),
            "{source}"
        );
    }
}

#[test]
fn planned_database_is_not_execution_and_goals_share_temporal_identity() {
    let c = dialogue::extract("Planejo usar MySQL no meu projeto", "demo");
    assert_eq!(c.len(), 1);
    assert_eq!(
        (&c[0].property[..], &c[0].value[..], &c[0].temporal[..]),
        ("database", "mysql", "planned")
    );
    let planned = dialogue::extract("Tenho como objetivo aprender francês", "demo");
    let completed = dialogue::extract("Já concluí meu objetivo de aprender francês", "demo");
    assert_eq!(planned[0].property, completed[0].property);
}

#[test]
fn equipment_paraphrases_preserve_subject_property_and_units() {
    for source in [
        "Meu computador principal está equipado com 36 GB de RAM",
        "O PC principal tem 36 GB de memória",
        "Tenho 36 GB de RAM no computador principal",
    ] {
        let c = dialogue::extract(source, "demo");
        assert_eq!(c.len(), 1, "{source}: {c:?}");
        assert_eq!(
            (&c[0].subject[..], &c[0].property[..], &c[0].value[..]),
            ("device.primary_computer", "ram_capacity", "36_gb")
        );
    }
}

#[test]
fn nominal_relations_cannot_take_authority_from_third_party_or_ambiguous_targets() {
    for source in [
        "My brother's own preference is coffee without sugar",
        "My own preference is maybe coffee without sugar",
        "My choice for my project database is SQLite or MySQL",
        "My choice for my project database is not SQLite",
        "Para o meu projeto Outro, a linguagem que decidi usar é Rust",
        "My favorite beverage for my colleague is tea",
        "My own preference is a recipe for coffee without sugar",
    ] {
        assert!(dialogue::extract(source, "demo").is_empty(), "{source}");
    }
}

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
fn typed_lexical_lookup_uses_canonical_links_after_short_correction_and_reopen() {
    let (d, mut s) = setup(true);
    for (source, kind) in [
        ("Meu computador principal tem 36 GB de RAM.", "hardware"),
        ("Meu notebook de trabalho tem 12 GB de RAM.", "hardware"),
        ("My choice for my project database is SQLite", "decision"),
        (
            "Estou começando um projeto de catálogo de minerais.",
            "project",
        ),
    ] {
        learn(&mut s, source, kind);
        for m in s
            .list("lucille", "demo")
            .unwrap()
            .into_iter()
            .filter(|m| m.state == Status::Candidate)
        {
            s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
                .unwrap();
        }
    }
    learn(&mut s, "Não tenho mais 36 GB, agora são 48 GB.", "hardware");
    for detail in s
        .memory_details("lucille", "demo")
        .unwrap()
        .into_iter()
        .filter(|d| d.memory.state == Status::Conflict)
    {
        let q = detail.quality.unwrap();
        s.resolve_conflict(
            &detail.memory.input.id,
            "lucille",
            "demo",
            detail.memory.version,
            q.replaces_version.unwrap(),
            &permit(),
        )
        .unwrap();
    }
    for m in s
        .list("lucille", "demo")
        .unwrap()
        .into_iter()
        .filter(|m| m.state == Status::Candidate)
    {
        s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
            .unwrap();
    }
    // More than the normal nomination cap of unrelated, newer active records.
    std::thread::sleep(std::time::Duration::from_secs(1));
    for index in 0..110 {
        let candidate = s
            .create_candidate(Candidate {
                id: uuid::Uuid::new_v4().to_string(),
                scope: "lucille".into(),
                project: "demo".into(),
                kind: Kind::Fact,
                content: format!("Unrelated synthetic item {index}"),
                fact_key: format!("synthetic.item{index}"),
                source: Source::Manual,
                source_id: format!("synthetic-item-{index}"),
            })
            .unwrap();
        s.change(
            &candidate.input.id,
            "lucille",
            "demo",
            candidate.version,
            Action::Approve,
        )
        .unwrap();
    }
    drop(s);
    let mut s = Store::open(&d.path().join("synthetic.sqlite3")).unwrap();
    for query in [
        "Qual a memória RAM do meu computador principal?",
        "Quanto de RAM tem meu PC principal?",
        "Qual a capacidade de memória do computador principal?",
        "Meu computador principal possui quantos gigabytes?",
        "Não estou dizendo que meu computador tem 128 GB de RAM.",
    ] {
        let c = s.context(request(query)).unwrap();
        assert!(
            c.facts.iter().any(|f| f.property == "ram_capacity"
                && f.entity == "device.primary_computer"
                && f.value == "48_gb"),
            "{query}: {:?}",
            c.facts
        );
        assert!(!c.facts.iter().any(|f| f.entity == "device.work_laptop"));
    }
    assert!(s
        .context(request(
            "Qual era o assunto do catálogo que eu estava fazendo?"
        ))
        .unwrap()
        .facts
        .iter()
        .any(|f| f.property == "description"));
    for query in [
        "Meu irmão usa qual banco de dados?",
        "Você lembra qual banco meu irmão usa?",
        "Você sabe qual é minha data de nascimento?",
        "Qual banco meu colega escolheu?",
    ] {
        assert!(
            s.context(request(query)).unwrap().facts.is_empty(),
            "{query}"
        );
    }
}

#[test]
fn renewal_after_restart_is_reviewed_and_does_not_erase_tombstones_or_infer_liking_transition() {
    for (original, forgotten, renewed) in [
        ("Eu gosto de jogos em que posso escolher minha classe.", "Esqueça minha preferência por escolher classes em jogos.", "Volte a guardar esta preferência: eu gosto de jogos em que posso escolher minha classe."),
        ("Prefiro café sem açúcar.", "Esqueça minha preferência por café.", "Volte a memorizar minha preferência por café sem açúcar."),
    ] {
        let (d, mut s) = setup(true);
        learn(&mut s, original, "preference");
        assert_eq!(s.list("lucille", "demo").unwrap().iter().filter(|m| m.state == Status::Active).count(), 1);
        s.enqueue_learning(input(forgotten, "forget")).unwrap();
        assert!(s.list("lucille", "demo").unwrap().is_empty());
        drop(s);
        let mut s = Store::open(&d.path().join("synthetic.sqlite3")).unwrap();
        learn(&mut s, original, "preference");
        assert!(s.list("lucille", "demo").unwrap().is_empty());
        learn(&mut s, renewed, "preference");
        let rows = s.list("lucille", "demo").unwrap();
        assert_eq!(rows.len(), 1, "{renewed}");
        assert_eq!(rows[0].state, Status::Candidate);
        assert_eq!(s.deleted_memories("lucille", "demo").unwrap().len(), 1);
        s.change(&rows[0].input.id,"lucille","demo",rows[0].version,Action::Approve).unwrap();
        assert!(s.context(request("Quais são minhas preferências?")).unwrap().facts.iter().any(|f| f.entity == "user" && f.modality == "preference"));
    }
    assert!(
        dialogue::extract("Voltei a gostar de café sem açúcar.", "demo")
            .iter()
            .all(|c| !c.rule.contains("renewal"))
    );
    assert!(
        dialogue::extract("Você lembra minha preferência por café sem açúcar?", "demo").is_empty()
    );
}

#[test]
fn project_purpose_is_project_description_in_both_current_source_and_approved_context() {
    let (_d, mut s) = setup(true);
    let text = "Estou começando um projeto de catálogo de conchas.";
    let source = s.begin_user_turn(input(text, "purpose-source")).unwrap();
    assert!(source["current_statements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f["property"] == "description" && f["modality"] == "project_description"));
    let proof = serde_json::from_value(source["proof"].clone()).unwrap();
    s.publish_user_turn(proof).unwrap();
    let lease = s.claim_llm_learning(now() + 1).unwrap().unwrap();
    s.complete_llm_learning(&lease, &output(text, "project"), now() + 2)
        .unwrap();
    for m in s.list("lucille", "demo").unwrap() {
        s.change(&m.input.id, "lucille", "demo", m.version, Action::Approve)
            .unwrap();
    }
    let c = s
        .context(request("Como está ficando aquele projeto?"))
        .unwrap();
    assert!(c
        .facts
        .iter()
        .any(|f| f.property == "description" && f.modality == "project_description"));
    assert!(c.facts.iter().all(|f| f.modality != "in_progress"));
}
