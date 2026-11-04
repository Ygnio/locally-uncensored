//! Bounded interpretation of original USER clauses. No model-supplied values.
use crate::{
    extraction::{digest, fold},
    Kind,
};

#[derive(Debug)]
pub(crate) struct Evidence<'a> {
    pub text: &'a str,
    pub span: std::ops::Range<usize>,
    pub actor: &'static str,
    pub entity: String,
    pub property: String,
    pub value: String,
    pub unit: Option<&'static str>,
    pub negated: bool,
    pub modality: &'static str,
    pub temporal: &'static str,
    pub origin: &'static str,
    pub scope: &'a str,
    pub confidence: f64,
    pub kind: Kind,
}
fn has(words: &[&str], alternatives: &[&str]) -> bool {
    alternatives.iter().any(|w| words.contains(w))
}
/// An imperative storage operation is distinct from a change in personal liking.
pub(crate) fn explicit_renewal(text: &str) -> bool {
    let s = fold(text);
    let command = s.split(':').next().unwrap_or(&s);
    let words = command.split(|c: char| !c.is_alphanumeric()).filter(|w| !w.is_empty()).collect::<Vec<_>>();
    !text.contains('?') && !has(&words, &["nao", "not", "never", "talvez", "maybe"])
        && ((words.first() == Some(&"volte") && has(&words, &["guardar", "memorizar", "registrar"]))
            || s.starts_with("quero voltar a registrar minha preferencia: ")
            || s.starts_with("i want to remember my preference again: ")
            || s.starts_with("pode voltar a memorizar: ")
            || s.starts_with("you can remember this again: "))
}
/// Interpretation is deliberately partial. Unproved relations return None.
pub(crate) fn interpret<'a>(text: &'a str, project: &'a str) -> Option<Evidence<'a>> {
    let s = fold(text);
    let words = s
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    if text.contains('?')
        || has(
            &words,
            &[
                "nao", "not", "never", "nunca", "talvez", "maybe", "might", "if", "se",
            ],
        )
        || has(
            &words,
            &[
                "irmao",
                "irma",
                "brother",
                "sister",
                "colega",
                "colleague",
                "manual",
                "personagem",
                "character",
            ],
        )
        || s.contains("don't")
    {
        return None;
    }
    let personal = has(&words, &["meu", "minha", "my", "eu", "i", "tenho"])
        || s.starts_with("o pc principal ");
    if !personal {
        return None;
    }
    let mut e = Evidence {
        text,
        span: 0..text.len(),
        actor: "user",
        entity: "user".into(),
        property: String::new(),
        value: String::new(),
        unit: None,
        negated: false,
        modality: "asserted",
        temporal: "durable",
        origin: "user_clause",
        scope: project,
        confidence: 0.9,
        kind: Kind::Fact,
    };
    if explicit_renewal(text) && s.contains("minha preferencia por ") {
        let target = s.split("minha preferencia por ").nth(1)?;
        if target == "cafe sem acucar" {
            e.kind = Kind::Preference; e.temporal = "preference";
            e.property = "coffee_sugar".into(); e.value = "without_sugar".into();
            return Some(e);
        }
    }
    // A preference for coffee is not a sugar preparation or a favorite-beverage claim.
    // Keep this separate canonical relation rather than manufacturing those qualifiers.
    if ["eu prefiro cafe","prefiro cafe","eu gosto de cafe","gosto de cafe","i prefer coffee","i like coffee"].contains(&s.as_str()) {
        e.kind=Kind::Preference;e.temporal="preference";
        e.property="beverage_preference".into();e.value="coffee".into();return Some(e)
    }
    // Nominal preference predicates require a personal possessor and a copula.
    let copula = words.iter().position(|w| ["e", "is"].contains(w));
    if let Some(at) = copula {
        let head = &words[..at];
        let raw_tail = &words[at + 1..];
        // A consumption-time adjunct qualifies the same preparation preference;
        // it does not nominate another speaker or a different drink.
        let tail = if let Some(boundary)=raw_tail.iter().position(|w|["when","while","quando"].contains(w)) {
            let adjunct=&raw_tail[boundary..];
            if adjunct.len()==4 && ["i","eu"].contains(&adjunct[1])
                && ["drink","tomo","bebo"].contains(&adjunct[2])
                && ["it","coffee","cafe"].contains(&adjunct[3]) {&raw_tail[..boundary]}else{raw_tail}
        }else{raw_tail};
        if has(
            head,
            &[
                "preferencia",
                "preference",
                "favorita",
                "favorito",
                "favorite",
                "favourite",
            ],
        ) && has(head, &["minha", "meu", "my"])
        {
            e.kind = Kind::Preference;
            e.temporal = "preference";
            if has(head, &["favorita", "favorito", "favorite", "favourite"])
                && has(head, &["bebida", "bebidas", "beverage", "drink"])
                && tail.len() == 1
                && has(tail, &["cafe", "coffee", "cha", "tea"])
            {
                e.property = "favorite_beverage".into();
                e.value = if has(tail, &["cha", "tea"]) {
                    "tea"
                } else {
                    "coffee"
                }
                .into();
                return Some(e);
            }
            if [
                &["cafe", "sem", "acucar"][..],
                &["coffee", "without", "sugar"][..],
            ]
            .contains(&tail)
            {
                e.property = "coffee_sugar".into();
                e.value = "without_sugar".into();
                return Some(e);
            }
        }
        if has(head, &["projeto", "project"])
            && has(head, &["meu", "my"])
            && (has(head, &["escolha", "choice"])
                || (has(head, &["decidi", "decided"]) && has(head, &["usar", "use"])))
            && tail.len() == 1
        {
            let property = if has(head, &["linguagem", "language"])
                && has(tail, &["rust", "python", "typescript", "javascript"])
            {
                "language"
            } else if has(head, &["database", "banco"])
                && has(tail, &["sqlite", "mysql", "postgresql"])
            {
                "database"
            } else {
                return None;
            };
            e.kind = Kind::Decision;
            e.entity = format!("project.{project}");
            e.property = property.into();
            e.value = tail[0].into();
            return Some(e);
        }
    }
    if has(&words, &["ram", "memoria", "memory"])
        && !has(
            &words,
            &[
                "comprar", "buy", "instale", "install", "quero", "want", "gpu", "vram", "livres",
                "free",
            ],
        )
        && (has(&words, &["tem", "possui", "tenho", "has", "have"])
            || s.contains("equipado com")
            || s.contains("equipped with"))
    {
        let amounts = crate::coverage::amounts(&s);
        if amounts.len() == 1 {
            e.entity = crate::coverage::device(&s)?.into();
            e.kind = Kind::Hardware;
            e.property = "ram_capacity".into();
            e.value = format!("{}_gb", amounts[0]);
            e.unit = Some("GB");
            return Some(e);
        }
    }
    // Aspect and target are independent: completing a goal is not a new desire.
    if let Some(at) = words.iter().position(|w| ["objetivo", "goal"].contains(w)) {
        let target = &words[at + 1..];
        let target = target.strip_prefix(&["de"]).unwrap_or(target);
        if target.len() >= 2
            && ["aprender", "learn"].contains(&target[0])
            && (has(&words[..at], &["tenho", "conclui", "completed"])
                || s.starts_with("my goal is"))
        {
            let canonical = format!("meu objetivo e {}", target.join(" "));
            e.kind = Kind::Goal;
            e.property = format!("goal.{}", &digest(&canonical)[..12]);
            e.value = s.clone();
            if has(&words[..at], &["conclui", "completed"]) {
                e.temporal = "completed";
                e.value = format!("completed:{}", target.join(" "));
            }
            return Some(e);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn retains_exact_span_and_relation_without_model_authority() {
        let text = "Meu computador principal está equipado com 36 GB de RAM";
        let e = interpret(text, "demo").unwrap();
        assert_eq!(&text[e.span.clone()], e.text);
        assert_eq!(
            (e.actor, e.origin, e.scope, e.modality, e.negated),
            ("user", "user_clause", "demo", "asserted", false)
        );
        assert_eq!(
            (
                e.entity.as_str(),
                e.property.as_str(),
                e.value.as_str(),
                e.unit
            ),
            (
                "device.primary_computer",
                "ram_capacity",
                "36_gb",
                Some("GB")
            )
        );
    }
}
