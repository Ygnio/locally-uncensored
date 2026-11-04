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
