//! Read-only instrumentation of the existing normalization boundaries. No Store/model.
use lucille_memory::{dialogue, llm};
use serde_json::{json, Value};
use std::io::Read;
fn main() {
    let mut input = String::new();
    std::io::stdin().take(1024 * 1024).read_to_string(&mut input).unwrap();
    let v: Value = serde_json::from_str(&input).unwrap();
    let text = v["text"].as_str().unwrap();
    let scope = v["scope"].as_str().unwrap();
    let project = v["project"].as_str().unwrap();
    let raw = v["output"].as_str().unwrap();
    let clauses = text.split(['.', '!', ';', '\n']).map(str::trim).filter(|s| !s.is_empty()).map(|span| {
        let claims = dialogue::extract(span, project);
        let promotion_proof = claims.iter().map(|c| json!({"claim":c,"safe_preference":dialogue::safe_preference(c)})).collect::<Vec<_>>();
        json!({"literal_span":span,"eligible_isolated":dialogue::eligible_source(span),"isolated_claims":promotion_proof})
    }).collect::<Vec<_>>();
    let result = json!({"original":text,"transport_author":"user","whole_eligible":dialogue::eligible_source(text),"whole_rules":dialogue::extract(text,project),"clauses":clauses,"structured_validation":match llm::validate(text,scope,project,raw) {Ok(c)=>json!({"accepted":c}),Err(e)=>json!({"error":e})}});
    println!("{}", serde_json::to_string(&result).unwrap());
}
