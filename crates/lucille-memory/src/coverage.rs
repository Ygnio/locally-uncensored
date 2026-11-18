//! Small, evidence-preserving extensions. No text rewriting or model authority.
use crate::{
    extraction::{digest, fold, Claim},
    Kind,
};
fn any(s: &str, patterns: &[&str]) -> bool {
    patterns.iter().any(|v| s.contains(v))
}
pub(crate) fn amounts(s: &str) -> Vec<u32> {
    let words = s
        .split(|c: char| !c.is_alphanumeric())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    let mut out = vec![];
    for (i, w) in words.iter().enumerate() {
        let value = if *w == "gb" && i > 0 {
            words[i - 1].parse().ok()
        } else {
            w.strip_suffix("gb").and_then(|v| v.parse().ok())
        };
        if let Some(n) = value.filter(|n: &u32| *n > 0 && *n <= 4096) {
            out.push(n)
        }
    }
    out
}
pub(crate) fn old_capacity(evidence: &str) -> Option<String> {
    let s = fold(evidence);
    if any(&s, &["nao tenho mais", "no longer have"]) && s.contains("agora") {
        amounts(s.split("agora").next()?)
            .first()
            .map(|n| format!("{n}_gb"))
    } else {
        None
    }
}
fn work_device(s: &str) -> bool {
    any(
        s,
        &[
            "notebook de trabalho",
            "notebook do trabalho",
            "note do trabalho",
            "work laptop",
        ],
    ) || crate::evidence::fronted_work_laptop(s)
}
pub(crate) fn device(s: &str) -> Option<&'static str> {
    let other = any(s, &["outro computador", "other computer"]);
    let work = work_device(s);
    let names_work = work || crate::evidence::names_fronted_work_laptop(s);
    let primary = any(
        s,
        &[
            "computador principal",
            "pc principal",
            "primary computer",
            "meu pc",
            "my pc",
            "meu computador",
            "my computer",
        ],
    );
    if other && names_work
        || primary && names_work
        || other && primary && !s.contains("meu outro computador")
    {
        return None;
    }
    if other {
        Some("device.other_computer")
    } else if work {
        Some("device.work_laptop")
    } else if primary {
        Some("device.primary_computer")
    } else {
        None
    }
}
pub(crate) fn mixed_devices(s: &str) -> bool {
    let other = any(s, &["outro computador", "other computer"]);
    let work = work_device(s) || crate::evidence::names_fronted_work_laptop(s);
    let primary = any(
        s,
        &[
            "computador principal",
            "pc principal",
            "primary computer",
            "meu pc",
            "my pc",
            "meu computador",
            "my computer",
        ],
    );
    (other && work) || (primary && (work || other))
}
fn claim(evidence: &str, kind: Kind, property: &str, value: &str, content: &str) -> Claim {
    Claim {
        authority: None,
        kind,
        content: content.into(),
        subject: "user".into(),
        property: property.into(),
        value: value.into(),
        confidence: 0.98,
        relevance: 0.9,
        temporal: "preference".into(),
        ttl_seconds: None,
        facets: vec![],
        evidence: evidence.into(),
        rule: "explicit_supported_preference".into(),
        project_hint: None,
        correction: false,
    }
}
pub(crate) fn supplement(evidence: &str) -> Option<Claim> {
    let s = fold(evidence);
    if any(
        &s,
        &[
            "imagine",
            "historia em que sou",
            "historia onde sou",
            "na ficcao",
            "pretend",
            "fictional narrator",
            "meu colega",
            "minha colega",
            "my friend",
            "se eu ",
            "if i ",
            "talvez",
            "maybe",
            "acho que",
        ],
    ) {
        return None;
    }
    if [
        "nao gosto mais de cafe",
        "eu nao gosto mais de cafe",
        "i no longer like coffee",
    ]
    .contains(&s.as_str())
    {
        let mut c = claim(
            evidence,
            Kind::Preference,
            "coffee_sugar",
            "dislikes_coffee",
            "Não gosta de café.",
        );
        c.correction = any(&s, &["mais", "no longer"]);
        return Some(c);
    }
    let coffee = any(&s, &["cafe", "coffee"]);
    let direct_choice = s.starts_with("no cafe, minha escolha ")
        || s.starts_with("minha escolha de cafe ")
        || s.starts_with("pra mim, cafe")
        || s.starts_with("para mim, cafe")
        || s.starts_with("coffee tastes best to me ");
    if coffee
        && direct_choice
        && !any(&s, &["nao ", " not ", "sem preferencia"])
        && any(
            &s,
            &[
                "sem acucar",
                "zero acucar",
                "sem adicao de acucar",
                "unsweetened",
            ],
        )
    {
        return Some(claim(
            evidence,
            Kind::Preference,
            "coffee_sugar",
            "without_sugar",
            "Prefere café sem açúcar.",
        ));
    }
    let choose = any(
        &s,
        &[
            "eu prefiro ",
            "prefiro ",
            "eu gosto ",
            "gosto ",
            "i prefer ",
            "i like ",
        ],
    ) && ["eu ", "prefiro ", "gosto ", "i "]
        .iter()
        .any(|v| s.starts_with(v));
    if (choose || s.starts_with("when i pick games, i go for "))
        && !any(&s, &["nao ", "don't ", "not ", "meu amigo"])
    {
        if any(
            &s,
            &["turn-based squad tactics", "jogos de tatica por turnos"],
        ) {
            return Some(claim(
                evidence,
                Kind::Preference,
                "games_squad_tactics",
                "positive",
                "Prefere jogos de tática de esquadrão por turnos.",
            ));
        }
        if any(&s, &["campanhas", "campaigns"])
            && any(
                &s,
                &[
                    "decisao altera a historia",
                    "decisoes alteram a historia",
                    "choices change the story",
                    "choices affect the story",
                ],
            )
        {
            return Some(claim(
                evidence,
                Kind::Preference,
                "games_branching_story",
                "positive",
                "Prefere campanhas com histórias alteradas pelas decisões.",
            ));
        }
    }
    // The correction direction follows the asserted current clause, never
    // whichever number happens to occur last.
    let contextual = crate::evidence::contextual_ram_replacement(evidence);
    if let Some((subject, current, previous)) =
        contextual.or_else(|| crate::evidence::replacement_ram(evidence))
    {
        let mut c = claim(
            evidence,
            Kind::Hardware,
            "ram_capacity",
            &format!("{current}_gb"),
            evidence,
        );
        c.subject = subject.into();
        c.temporal = "durable".into();
        c.confidence = 0.85;
        c.correction = true;
        let rule = if contextual.is_some() {
            crate::evidence::CONTEXTUAL_RAM_CORRECTION_RULE
        } else {
            "explicit_ram_correction_from_"
        };
        c.rule = format!("{rule}{previous}_gb");
        return Some(c);
    }
    // Existing old -> new correction without a named first-clause device.
    if let Some(old) = old_capacity(evidence) {
        let newer = amounts(s.split("agora").nth(1)?);
        if newer.len() != 1 {
            return None;
        }
        let mut c = claim(
            evidence,
            Kind::Hardware,
            "ram_capacity",
            &format!("{}_gb", newer[0]),
            evidence,
        );
        c.subject = device(&s).unwrap_or("device.unspecified").into();
        c.temporal = "durable".into();
        c.confidence = 0.85;
        c.correction = true;
        c.rule = format!("explicit_ram_correction_from_{old}");
        return Some(c);
    }
    let sizes = amounts(&s);
    let update = any(
        &s,
        &["atualizei ", "troquei a ram", "upgraded ", "updated "],
    );
    let named = device(&s);
    if sizes.len() == 1
        && named.is_some()
        && (any(&s, &["ram", "memoria", "memory"]) || update)
        && !any(&s, &["nao ", "not ", "free", "livre"])
    {
        let attribution = s.starts_with("meu ")
            || s.starts_with("my ")
            || s.starts_with("atualizei ")
            || s.starts_with("eu atualizei ")
            || s.starts_with("i upgraded ");
        if attribution && (update || any(&s, &[" tem ", " possui ", " has "])) {
            let subject = named?;
            let mut c = claim(
                evidence,
                Kind::Hardware,
                if any(&s, &["ram", "memoria", "memory"]) {
                    "ram_capacity"
                } else {
                    "capacity_update"
                },
                &format!("{}_gb", sizes[0]),
                evidence,
            );
            c.subject = subject.into();
            c.temporal = "durable".into();
            c.rule = "explicit_named_device_capacity".into();
            c.correction = update;
            c.confidence = 0.9;
            return Some(c);
        }
    }
    if let Some(target) = s.strip_prefix("conclui meu objetivo de ") {
        if target.len() < 4 {
            return None;
        }
        let original = format!("meu objetivo e {target}");
        let mut c = claim(
            evidence,
            Kind::Goal,
            &format!("goal.{}", &digest(&original)[..12]),
            &format!("completed:{target}"),
            &format!("Objetivo concluído: {target}."),
        );
        c.temporal = "completed".into();
        c.correction = true;
        c.confidence = 0.9;
        c.rule = "explicit_completed_goal".into();
        return Some(c);
    }
    None
}
