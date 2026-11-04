//! V2 intermediate representation. The model identifies complete literal claims;
//! independent interpretation owns entities, values, temporal state and storage keys.
//! No proposal means no extraction: this is not a rule fallback for LLM omissions.
use crate::{
    extraction::{extract, fold},
    intelligent::Claim,
    llm::eligible_source,
    Kind, Result,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtomicOutput {
    pub version: u32,
    pub scope: String,
    pub project: String,
    pub claims: Vec<AtomicClaim>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AtomicClaim {
    pub evidence: String,
    pub category: Kind,
}
#[derive(Debug, Serialize)]
pub struct Decision {
    pub evidence: String,
    pub proposed_category: Kind,
    pub accepted: Option<Claim>,
    pub reason: String,
}
#[derive(Debug, Serialize)]
pub struct NormalizationReport {
    pub version: u32,
    pub decisions: Vec<Decision>,
}

pub fn normalize(text: &str, scope: &str, project: &str, raw: &str) -> Result<NormalizationReport> {
    if !eligible_source(text) || raw.len() > 12000 {
        return Err("Ineligible extraction source or response size".into());
    }
    let output: AtomicOutput =
        serde_json::from_str(raw).map_err(|_| "Invalid atomic extraction schema")?;
    if output.version != 2
        || output.scope != scope
        || output.project != project
        || output.claims.len() > 4
    {
        return Err("Atomic version, scope, project or claim ceiling mismatch".into());
    }
    let mut report = NormalizationReport {
        version: 2,
        decisions: vec![],
    };
    let mut seen = std::collections::HashSet::new();
    for proposal in output.claims {
        let result = interpret(text, project, &proposal);
        let (accepted, reason) = match result {
            Ok(claim) if seen.insert(claim.fingerprint()) => (
                Some(claim),
                "Complete literal evidence independently interpreted; review required".into(),
            ),
            Ok(_) => (None, "Duplicate grounded claim".into()),
            Err(reason) => (None, reason),
        };
        report.decisions.push(Decision {
            evidence: proposal.evidence,
            proposed_category: proposal.category,
            accepted,
            reason,
        });
    }
    Ok(report)
}
fn interpret(text: &str, project: &str, proposal: &AtomicClaim) -> Result<Claim> {
    let original = proposal.evidence.trim();
    let span = original.trim_end_matches(['.', '!', ';', '\n']).trim();
    if original.len() > 300
        || span.len() < 12
        || !text.contains(original)
        || !text
            .split(['.', '!', ';', '\n'])
            .any(|sentence| sentence.trim() == span)
    {
        return Err(
            "Incomplete or nonliteral evidence; missing words are never reconstructed".into(),
        );
    }
    let mut claims = extract(span);
    if claims.is_empty() {
        if let Some(claim) = game_preference(span) {
            claims.push(claim)
        }
    }
    if claims.len() != 1 {
        return Err("Unsupported or ambiguous complete claim".into());
    }
    let mut claim = claims.into_iter().next().unwrap();
    // 'fact' is a documented broad category alias only for independently
    // established hardware/project observations. Never turn uncertainty into fact.
    let alias =
        proposal.category == Kind::Fact && matches!(claim.kind, Kind::Hardware | Kind::Project);
    if claim.kind != proposal.category && !alias {
        return Err("Category differs from independently supported interpretation".into());
    }
    if claim
        .project_hint
        .as_ref()
        .is_some_and(|p| fold(p) != fold(project))
    {
        return Err("Explicit project differs from bound project".into());
    }
    if claim.kind == Kind::Hardware
        && claim.subject != "device.unspecified"
        && crate::coverage::device(&fold(span)).is_none()
    {
        return Err("No unambiguous named equipment; do not assume the primary device".into());
    }
    // Never make the promotion policy more permissive: V2 proposals require
    // review even when a deterministic rule could have auto-promoted a preference.
    claim.confidence = claim.confidence.min(0.9);
    claim.relevance = claim.relevance.min(0.8);
    claim.rule = format!("llm_v2_grounded:{}", claim.rule);
    Ok(claim)
}

// A bounded independent interpretation for explicit game-feature preferences.
// Full folded evidence is the value; no feature or polarity is inferred.
fn game_preference(span: &str) -> Option<Claim> {
    let s = fold(span);
    if ![
        "prefiro ",
        "eu prefiro ",
        "gosto ",
        "eu gosto ",
        "adoro ",
        "eu adoro ",
        "i prefer ",
        "i like ",
    ]
    .iter()
    .any(|p| s.starts_with(p))
        || ["nao ", "not ", "don't", "talvez", "maybe", "mas ", "but "]
            .iter()
            .any(|p| s.contains(p))
        || !["jogos", "games", "rpg"].iter().any(|p| s.contains(p))
    {
        return None;
    }
    let classes = s.contains("classes") && ["personage", "character"].iter().any(|p| s.contains(p));
    let progression = ["evolucao", "progressao", "progression"]
        .iter()
        .any(|p| s.contains(p));
    let property = if classes {
        "games_classes"
    } else if progression {
        "games_progression"
    } else {
        return None;
    };
    let mut facets = vec![];
    if classes {
        facets.push("classes".into())
    }
    if progression {
        facets.push("progression".into())
    }
    Some(Claim {
        authority: None,
        kind: Kind::Preference,
        content: span.into(),
        subject: "user".into(),
        property: property.into(),
        value: s,
        confidence: 0.9,
        relevance: 0.8,
        temporal: "preference".into(),
        ttl_seconds: None,
        facets,
        evidence: span.into(),
        rule: "explicit_game_features".into(),
        project_hint: None,
        correction: false,
    })
}
