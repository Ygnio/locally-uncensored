use lucille_memory::{intelligent::*, llm::*, Kind};
use sha2::Digest;
#[test]
fn real_evaluation_corpora_remain_sealed() {
    let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    for name in ["phase21c-real-final", "phase21c-real-retrieval"] {
        let bytes = std::fs::read(base.join(format!("tests/fixtures/{name}.json"))).unwrap();
        let manifest: serde_json::Value = serde_json::from_slice(
            &std::fs::read(base.join(format!("tests/fixtures/{name}.manifest.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(
            format!("{:x}", sha2::Sha256::digest(&bytes)),
            manifest["sha256"].as_str().unwrap()
        );
    }
}
#[test]
fn opaque_storage_key_alias_never_overrides_semantic_grounding() {
    let text = "Meu objetivo é estudar italiano";
    let mut p = serde_json::json!({"kind":"goal","content":text,"entity":"user","property":"goal","value":"meu objetivo e estudar italiano","scope":"lucille","project":"atlas","temporal":"durable","evidence":text,"correction":false,"hypothesis":false,"confidence":0.99,"relevance":0.9});
    let output = |p: &serde_json::Value| serde_json::json!({"proposals":[p]}).to_string();
    let accepted = validate(text, "lucille", "atlas", &output(&p)).unwrap();
    assert_eq!(accepted, extract(text));
    assert_eq!(accepted[0].kind, Kind::Goal);
    p["value"] = serde_json::json!("meu objetivo e praticar medicina");
    assert!(validate(text, "lucille", "atlas", &output(&p)).is_err());
    p["value"] = serde_json::json!("meu objetivo e estudar italiano");
    p["entity"] = serde_json::json!("other_user");
    assert!(validate(text, "lucille", "atlas", &output(&p)).is_err());
    p["entity"] = serde_json::json!("user");
    p["correction"] = serde_json::json!(true);
    assert!(validate(text, "lucille", "atlas", &output(&p)).is_err());
    p["correction"] = serde_json::json!(false);
    assert!(validate(
        "Meu colega escreveu: Meu objetivo é estudar italiano",
        "lucille",
        "atlas",
        &output(&p)
    )
    .is_err());
}
