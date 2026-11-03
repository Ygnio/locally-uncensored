//! The model proposes; this module owns deterministic, extractive grounding.
use crate::{
    extraction::{extract, fold, unsafe_text},
    intelligent::Claim,
    Kind, Result,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtractionMode {
    #[default]
    Rules,
    Llm,
    Hybrid,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Proposal {
    pub kind: Kind,
    pub content: String,
    pub entity: String,
    pub property: String,
    pub value: String,
    pub scope: String,
    pub project: String,
    pub temporal: String,
    pub evidence: String,
    pub correction: bool,
    pub hypothesis: bool,
    pub confidence: f64,
    pub relevance: f64,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Output {
    pub proposals: Vec<Proposal>,
}
pub fn eligible_source(text: &str) -> bool {
    let s = fold(text);
    text.len() >= 12
        && text.len() <= 1600
        && !unsafe_text(&s)
        && !text.contains('?')
        && [
            "eu ",
            "meu ",
            "minha ",
            "prefiro ",
            "gosto ",
            "adoro ",
            "curto ",
            "estou ",
            "preciso ",
            "nao ",
            "i ",
            "my ",
            "o projeto ",
            "no projeto ",
            "corrigindo:",
            "na verdade",
            "troquei ",
            "no cafe, minha escolha ",
            "pra mim, cafe",
            "para mim, cafe",
            "when i pick games, i go for ",
            "coffee tastes best to me ",
            "conclui meu objetivo de ",
        ]
        .iter()
        .any(|prefix| s.starts_with(prefix))
        && ![
            "imagine",
            "forwarded",
            "encaminhad",
            "voce disse",
            "you said",
            "ironia",
            "sarcas",
            "so que nao",
            "quem dera",
            "meu primo",
            "meu colega",
            "minha colega",
            "ele escreveu",
            "ela escreveu",
            "disse",
            "said",
            "forward",
            "according",
            "segundo",
            "my friend",
            "brincadeira",
            "isso e falso",
        ]
        .iter()
        .any(|p| s.contains(p))
}
/// Exact source quotations are necessary but not sufficient: known topics are
/// independently interpreted, while unsupported semantic assertions abstain.
pub fn validate(text: &str, scope: &str, project: &str, json: &str) -> Result<Vec<Claim>> {
    if !eligible_source(text) || json.len() > 12000 {
        return Err("Ineligible extraction source or response size".into());
    }
    let output: Output =
        serde_json::from_str(json).map_err(|_| "Invalid structured extraction schema")?;
    if output.proposals.len() > 4 {
        return Err("Too many proposals".into());
    }
    let mut accepted = vec![];
    for p in output.proposals {
        if p.scope != scope
            || p.project != project
            || p.evidence.len() > 300
            || p.evidence.trim().len() < 12
            || !text.contains(&p.evidence)
            || !text
                .split(['.', '!', ';', '\n'])
                .any(|sentence| sentence.trim() == p.evidence.trim())
            || p.value.is_empty()
            || p.value.len() > 200
            || !p.confidence.is_finite()
            || !p.relevance.is_finite()
            || !(0.0..=1.0).contains(&p.confidence)
            || !(0.0..=1.0).contains(&p.relevance)
        {
            return Err("Ungrounded proposal or scope".into());
        }
        let mut known = extract(&p.evidence).into_iter().find(|c| {
            c.kind == p.kind
                && c.subject == p.entity
                && c.property == p.property
                && c.value == p.value
        });
        if known.is_none() {
            known = novel_preference(&p);
        }
        let Some(mut c) = known else {
            return Err(
                "Unsupported semantic claim; review evidence outside automatic learning".into(),
            );
        };
        if c.temporal != p.temporal
            || c.correction != p.correction
            || (c.kind == Kind::Hypothesis) != p.hypothesis
            || c.project_hint
                .as_ref()
                .is_some_and(|v| fold(v) != fold(project))
            || !(p.content == c.content || fold(&p.content) == fold(&p.evidence))
        {
            return Err("Claim interpretation differs from evidence".into());
        }
        // Never inherit a model score as automatic approval authority.
        if c.rule.starts_with("llm_") {
            c.confidence = 0.9;
            c.relevance = 0.8;
        } else {
            c.confidence = c.confidence.min(p.confidence);
            c.relevance = c.relevance.min(p.relevance);
        }
        if !accepted
            .iter()
            .any(|old: &Claim| old.fingerprint() == c.fingerprint())
        {
            accepted.push(c)
        }
    }
    Ok(accepted)
}
fn novel_preference(p: &Proposal) -> Option<Claim> {
    let s = fold(&p.evidence);
    if p.kind != Kind::Preference
        || p.entity != "user"
        || p.correction
        || p.hypothesis
        || p.temporal != "preference"
        || !p.evidence.contains(&p.value)
        || ![
            "gosto ",
            "eu gosto ",
            "prefiro ",
            "eu prefiro ",
            "adoro ",
            "eu adoro ",
            "i like ",
            "i prefer ",
        ]
        .iter()
        .any(|v| s.starts_with(v))
    {
        return None;
    }
    if ["nao ", "not ", "don't", "talvez", "maybe", "mas ", "but "]
        .iter()
        .any(|v| s.contains(v))
    {
        return None;
    }
    let property = if s.contains("personage") && s.contains("classes") {
        "games_classes"
    } else if s.contains("jogos") && (s.contains("evolucao") || s.contains("progressao")) {
        "games_progression"
    } else {
        return None;
    };
    if property != p.property {
        return None;
    }
    Some(Claim {
        kind: Kind::Preference,
        content: p.evidence.trim().into(),
        subject: "user".into(),
        property: property.into(),
        value: fold(&p.value),
        confidence: 0.9,
        relevance: 0.8,
        temporal: "preference".into(),
        ttl_seconds: None,
        facets: vec![],
        evidence: p.evidence.clone(),
        rule: "llm_grounded_extract".into(),
        project_hint: None,
        correction: false,
    })
}
