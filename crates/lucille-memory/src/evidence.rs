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
fn tokens(s: &str) -> Vec<&str> {
    s.split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect()
}
fn possessive(word: &str) -> bool {
    ["meu", "minha", "meus", "minhas", "my"].contains(&word)
}
fn preference_descriptor(words: &[&str]) -> bool {
    if has(
        words,
        &[
            "contrario",
            "oposto",
            "opposite",
            "except",
            "exceto",
            "instead",
            "rather",
        ],
    ) {
        return false;
    }
    words.iter().enumerate().any(|(i, word)| {
        let prefix = &words[..i];
        let after = &words[i + 1..];
        let emphasis = after
            .first()
            .is_some_and(|w| ["own", "proprio", "propria", "proprios", "proprias"].contains(w));
        possessive(word)
            && after.get(usize::from(emphasis)).is_some_and(|w| {
                [
                    "preferencia",
                    "preferencias",
                    "preference",
                    "preferences",
                    "preferred",
                    "favorito",
                    "favorita",
                    "favorite",
                    "favourite",
                ]
                .contains(w)
            })
            && (matches!(
                prefix,
                [] | ["a" | "o" | "as" | "os" | "among"]
                    | ["one", "of"]
                    | ["entre", "as", "bebidas"]
                    | ["among", "the", "drinks" | "beverages"]
            ) || (prefix.last() == Some(&"de")
                && has(prefix, &["idioma", "lingua", "tema", "respostas"])))
    })
}
fn nominal_preference<'a>(words: &'a [&'a str]) -> Option<(&'a [&'a str], &'a [&'a str])> {
    for (i, word) in words.iter().enumerate() {
        if ["e", "sao", "is", "are"].contains(word) {
            let left = &words[..i];
            let right = &words[i + 1..];
            if preference_descriptor(left) {
                return Some((left, right));
            }
            if preference_descriptor(right) {
                return Some((right, left));
            }
        }
        let tail = &words[i..];
        if tail.len() >= 5
            && ["faz", "fazem"].contains(word)
            && tail[1] == "parte"
            && ["de", "da", "das", "do", "dos"].contains(&tail[2])
            && preference_descriptor(&tail[3..])
        {
            return Some((&tail[3..], &words[..i]));
        }
    }
    None
}
fn adverbial_preference(words: &[&str]) -> Option<(usize, usize)> {
    words.iter().enumerate().find_map(|(i, word)| {
        if *word != "i" {
            return None;
        }
        let modifier = words[i + 1..]
            .iter()
            .take(2)
            .take_while(|w| ["now", "personally", "usually", "really", "generally"].contains(w))
            .count();
        let verb = i + 1 + modifier;
        (modifier > 0
            && words
                .get(verb)
                .is_some_and(|w| ["prefer", "like", "enjoy", "love"].contains(w)))
        .then_some((i, verb + 1))
    })
}
/// Witness the speaker of the predicate without deciding its canonical value.
pub(crate) fn owns_preference_predicate(text: &str) -> bool {
    let s = fold(text);
    let words = tokens(&s);
    nominal_preference(&words).is_some()
        || adverbial_preference(&words).is_some()
        || explicit_renewal(text)
}
fn preference_value(descriptor: &[&str], target: &[&str]) -> Option<(&'static str, &'static str)> {
    let start = target
        .iter()
        .take_while(|w| ["a", "o", "os", "the"].contains(w))
        .count();
    let target = &target[start..];
    let target = if let Some(at) = target
        .iter()
        .position(|w| ["when", "while", "quando"].contains(w))
    {
        let adjunct = &target[at..];
        if adjunct.len() == 4
            && ["i", "eu"].contains(&adjunct[1])
            && ["drink", "tomo", "bebo"].contains(&adjunct[2])
            && ["it", "coffee", "cafe"].contains(&adjunct[3])
        {
            &target[..at]
        } else {
            target
        }
    } else {
        target
    };
    match target {
        ["light", "theme"] | ["tema", "claro"] => Some(("interface_theme", "light")),
        ["dark", "theme"] | ["tema", "escuro"] => Some(("interface_theme", "dark")),
        ["portugues" | "portuguese"]
            if has(
                descriptor,
                &[
                    "idioma",
                    "lingua",
                    "language",
                    "respostas",
                    "answers",
                    "replies",
                ],
            ) =>
        {
            Some(("response_language", "pt"))
        }
        ["jogos", "de", "estrategia", "por", "turnos"] | ["turn", "based", "strategy", "games"] => {
            Some(("games_turn_strategy", "positive"))
        }
        ["cafe" | "coffee", preparation, "acucar" | "sugar"] => match *preparation {
            "sem" | "without" => Some(("coffee_sugar", "without_sugar")),
            "com" | "with" => Some(("coffee_sugar", "with_sugar")),
            _ => None,
        },
        _ => None,
    }
}
fn modified_preference_value(words: &[&str]) -> Option<(&'static str, &'static str)> {
    let (speaker, target) = adverbial_preference(words)?;
    let before = &words[..speaker];
    let target = &words[target..];
    if before.windows(2).any(|pair| pair == ["my", "coffee"])
        || before
            .windows(3)
            .any(|pair| pair == ["my", "own", "coffee"])
    {
        // Only the explicitly bound coffee permits this pronoun resolution.
        if let ["it", preparation, "sugar"] = target {
            return preference_value(&[], &["coffee", preparation, "sugar"]);
        }
    }
    preference_value(&[], target)
}
fn device_relative_clause(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(i, word)| {
        let body = match *word {
            "que" => words[i + 1..]
                .strip_prefix(&["eu"])
                .unwrap_or(&words[i + 1..]),
            "i" => &words[i + 1..],
            _ => return false,
        };
        body.first()
            .is_some_and(|verb| ["uso", "utilizo", "use"].contains(verb))
            && [
                &["computador", "principal"][..],
                &["pc", "principal"],
                &["notebook", "de", "trabalho"],
                &["notebook", "do", "trabalho"],
                &["primary", "computer"],
                &["work", "laptop"],
            ]
            .iter()
            .any(|device| words[..i].ends_with(device))
    })
}
pub(crate) fn owns_device_relative_clause(text: &str) -> bool {
    device_relative_clause(&tokens(&fold(text)))
}
fn possessed_device(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(i, word)| {
        possessive(word)
            && words[i + 1..]
                .iter()
                .take(3)
                .find(|w| !["outro", "other", "primary", "main", "work"].contains(w))
                .is_some_and(|w| {
                    ["computador", "computer", "pc", "notebook", "laptop", "note"].contains(w)
                })
    })
}
fn installed_capacity_predicate(words: &[&str]) -> bool {
    let Some(amount_at) = words.iter().enumerate().position(|(i, word)| {
        (words.get(i + 1) == Some(&"gb") && word.parse::<u32>().is_ok())
            || word
                .strip_suffix("gb")
                .is_some_and(|n| n.parse::<u32>().is_ok())
    }) else {
        return false;
    };
    words.iter().enumerate().any(|(i, word)| {
        let next = i
            + 1
            + words[i + 1..]
                .iter()
                .take_while(|w| {
                    ["hoje", "atualmente", "agora", "today", "currently", "now"].contains(w)
                })
                .count();
        match *word {
            "ha" => next == amount_at,
            "conta" => words.get(next) == Some(&"com") && next + 1 == amount_at,
            "esta" | "estao" | "is" | "are" => {
                words.get(next).is_some_and(|w| {
                    [
                        "instalado",
                        "instalados",
                        "instalada",
                        "instaladas",
                        "installed",
                    ]
                    .contains(w)
                }) && (next + 1 == amount_at
                    || (amount_at < i
                        && words[amount_at + 1..i]
                            .iter()
                            .all(|w| ["gb", "of", "de", "ram", "memoria", "memory"].contains(w))))
            }
            _ => false,
        }
    })
}
fn owned_choice(words: &[&str]) -> bool {
    words.iter().enumerate().any(|(at, word)| {
        possessive(word)
            && words[at + 1..]
                .iter()
                .take(3)
                .find(|w| !["database", "language", "final", "atual", "current"].contains(w))
                .is_some_and(|w| ["escolha", "choice", "decisao", "decision"].contains(w))
    }) || (has(words, &["meu", "my"])
        && has(words, &["decidi", "decided"])
        && has(words, &["usar", "use"]))
}

pub(crate) fn owns_project_choice_descriptor(text: &str) -> bool {
    let s = fold(text);
    let words = tokens(&s);
    has(&words, &["projeto", "project"]) && owned_choice(&words)
}

fn project_choice(words: &[&str]) -> Option<(&'static str, &'static str)> {
    if !has(words, &["projeto", "project"])
        || has(
            words,
            &[
                "anterior",
                "antiga",
                "previous",
                "former",
                "candidata",
                "candidate",
            ],
        )
    {
        return None;
    }
    for (i, word) in words.iter().enumerate() {
        if !["e", "foi", "is", "was"].contains(word) {
            continue;
        }
        for (descriptor, target) in [
            (&words[..i], &words[i + 1..]),
            (&words[i + 1..], &words[..i]),
        ] {
            if !owned_choice(descriptor) {
                continue;
            }
            let Some((last, qualifier)) = target.split_last() else {
                continue;
            };
            // An introductory project adjunct may precede the selected value.
            // Alternative values or an additional proposition do not qualify.
            if !qualifier.is_empty()
                && !qualifier
                    .last()
                    .is_some_and(|w| ["projeto", "project"].contains(w))
            {
                continue;
            }
            for (property, nouns, values) in [
                (
                    "language",
                    &["linguagem", "language"][..],
                    &["rust", "python", "typescript", "javascript"][..],
                ),
                (
                    "database",
                    &["database", "banco"][..],
                    &["sqlite", "mysql", "postgresql"][..],
                ),
            ] {
                if has(descriptor, nouns) {
                    if let Some(value) = values.iter().find(|v| *v == last) {
                        return Some((property, *value));
                    }
                }
            }
        }
    }
    None
}
/// An imperative storage operation is distinct from a change in personal liking.
pub(crate) fn explicit_renewal(text: &str) -> bool {
    let s = fold(text);
    let command = s.split(':').next().unwrap_or(&s);
    let words = command
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    !text.contains('?')
        && !has(&words, &["nao", "not", "never", "talvez", "maybe"])
        && ((words.first() == Some(&"volte")
            && has(&words, &["guardar", "memorizar", "registrar"]))
            || s.starts_with("quero voltar a registrar minha preferencia: ")
            || s.starts_with("i want to remember my preference again: ")
            || s.starts_with("pode voltar a memorizar: ")
            || s.starts_with("you can remember this again: "))
}
/// Interpretation is deliberately partial. Unproved relations return None.
pub(crate) fn interpret<'a>(text: &'a str, project: &'a str) -> Option<Evidence<'a>> {
    let s = fold(text);
    let words = tokens(&s);
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
    let relative_device = device_relative_clause(&words);
    let personal = has(
        &words,
        &["meu", "minha", "meus", "minhas", "my", "eu", "i", "tenho"],
    ) || relative_device
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
            e.kind = Kind::Preference;
            e.temporal = "preference";
            e.property = "coffee_sugar".into();
            e.value = "without_sugar".into();
            return Some(e);
        }
    }
    // A preference for coffee is not a sugar preparation or a favorite-beverage claim.
    // Keep this separate canonical relation rather than manufacturing those qualifiers.
    if [
        "eu prefiro cafe",
        "prefiro cafe",
        "eu gosto de cafe",
        "gosto de cafe",
        "i prefer coffee",
        "i like coffee",
    ]
    .contains(&s.as_str())
    {
        e.kind = Kind::Preference;
        e.temporal = "preference";
        e.property = "beverage_preference".into();
        e.value = "coffee".into();
        return Some(e);
    }
    if let Some((property, value)) = nominal_preference(&words)
        .and_then(|(descriptor, target)| preference_value(descriptor, target))
        .or_else(|| modified_preference_value(&words))
    {
        e.kind = Kind::Preference;
        e.temporal = "preference";
        e.property = property.into();
        e.value = value.into();
        return Some(e);
    }
    // Nominal preference predicates require a personal possessor and a copula.
    let copula = words.iter().position(|w| ["e", "is"].contains(w));
    if let Some(at) = copula {
        let head = &words[..at];
        let tail = &words[at + 1..];
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
        }
    }
    if let Some((property, value)) = project_choice(&words) {
        e.kind = Kind::Decision;
        e.entity = format!("project.{project}");
        e.property = property.into();
        e.value = value.into();
        return Some(e);
    }
    if has(&words, &["ram", "memoria", "memory"])
        && !has(
            &words,
            &[
                "comprar", "buy", "instale", "install", "quero", "want", "gpu", "vram", "livres",
                "free", "livre", "gostaria", "would", "should", "deveria", "pretendo", "plan",
                "previsao", "previsto", "prevista", "maximum", "maximo",
            ],
        )
        && (has(&words, &["tem", "possui", "tenho", "has", "have"])
            || s.contains("equipado com")
            || s.contains("equipped with")
            || ((possessed_device(&words) || relative_device)
                && installed_capacity_predicate(&words)))
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
