//! Protocol V3: literal conversational claims, bounded interpretation and risk proof.
//! It does not call a model, infer hidden facts or grant database/tool authority.
use crate::normalization::AtomicOutput;
use crate::{
    extraction::{digest, fold, unsafe_text},
    intelligent::Claim,
    Kind, Result,
};
fn any(s: &str, p: &[&str]) -> bool {
    p.iter().any(|v| s.contains(v))
}
pub fn eligible_source(text: &str) -> bool {
    let folded = fold(text);
    // Source framing and denial of authorship dominate any embedded first-person
    // preference. A proposal cannot strip these qualifiers to gain authority.
    let tokens = folded
        .split(|c: char| !c.is_alphanumeric())
        .collect::<Vec<_>>();
    if tokens.iter().any(|w| {
        w.starts_with("fictic")
            || w.starts_with("ficc")
            || w.starts_with("fiction")
            || w.starts_with("personag")
            || w.starts_with("citad")
            || w.starts_with("citac")
            || [
                "finja",
                "pretend",
                "system",
                "sistema",
                "developer",
                "token",
                "tokens",
                "segredo",
                "secret",
                "jamais",
            ]
            .contains(w)
    }) || any(
        &folded,
        &[
            "ignore ",
            "nao e verdade",
            "not true",
            "nao disse",
            "nao afirmei",
            "nunca afirmei",
            "never said",
            "did not say",
            "didn't say",
            "nao registre",
            "do not record",
            "no texto",
            "no roteiro",
            "na historia",
            "na narrativa",
            "log de ferramenta",
            "tool output",
            "tool log",
        ],
    ) {
        return false;
    }

    if any(
        &folded,
        &[
            " disse ",
            " said ",
            " diz que ",
            " says ",
            "se eu ",
            "if i ",
            "meu sobrinho",
            "minha sobrinha",
            "meu filho",
            "minha filha",
            "meu tio",
            "minha tia",
            "minha esposa",
            "meu esposo",
            "my son",
            "my daughter",
            "my wife",
            "my husband",
            "my teacher",
            "my doctor",
        ],
    ) {
        return false;
    }
    let words = folded.split_whitespace().collect::<Vec<_>>();
    if words.len() > 1
        && ["meu", "minha", "my"].contains(&words[0])
        && ![
            "objetivo",
            "goal",
            "pc",
            "computador",
            "notebook",
            "note",
            "outro",
            "other",
            "work",
            "primary",
            "idioma",
            "lingua",
            "language",
            "preferencia",
            "preference",
            "interface",
            "escolha",
            "favorite",
            "preferred",
            "respostas",
            "answers",
            "replies",
            "tema",
            "theme",
            "comunicacao",
        ]
        .contains(&words[1])
    {
        return false;
    }

    (12..=1600).contains(&text.len())
        && !unsafe_text(&fold(text))
        && !any(
            &fold(text),
            &[
                "ironia",
                "sarcas",
                "brincadeira",
                "quem dera",
                "my neighbour",
                "meu primo",
                "she says",
                "he says",
                "ela afirma",
                "ele afirma",
                "faz de conta",
            ],
        )
        && text
            .split(['.', '!', ';', '\n'])
            .any(|s| s.trim().len() >= 12 && !s.contains('?'))
}
pub fn project_scope_matches(text: &str, project: &str) -> bool {
    let s = fold(text);
    let w = s
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
        .collect::<Vec<_>>();
    !w.windows(2).any(|p| {
        ["project", "projeto"].contains(&p[0])
            && !p[1].is_empty()
            && ![
                "de", "do", "da", "novo", "new", "pessoal", "personal", "is", "e", "usa", "uses",
                "utiliza", "requires", "requer", "precisa", "needs", "aqui", "here", "going",
            ]
            .contains(&p[1])
            && p[1] != fold(project)
    })
}
fn base(
    span: &str,
    kind: Kind,
    subject: &str,
    property: &str,
    value: &str,
    temporal: &str,
) -> Claim {
    Claim {
        kind,
        content: span.into(),
        subject: subject.into(),
        property: property.into(),
        value: value.into(),
        confidence: 0.9,
        relevance: 0.8,
        temporal: temporal.into(),
        ttl_seconds: None,
        facets: vec![],
        evidence: span.into(),
        rule: "explicit_v3_grounded".into(),
        project_hint: None,
        correction: any(
            &fold(span),
            &[
                "na verdade",
                "agora prefiro",
                "mudei de ideia",
                "actually",
                "now i prefer",
                "i now prefer",
                "decidi trocar",
                "troquei ",
            ],
        ),
    }
}
fn ambiguous_preference(s: &str) -> bool {
    any(
        s,
        &[
            "sem classes",
            "without classes",
            "sem evolucao",
            "without progression",
            "nao apenas",
            "not only",
            "as vezes",
            "sometimes",
            "talvez",
            "maybe",
            "might prefer",
        ],
    )
}
fn transient(s: &str) -> bool {
    any(
        s,
        &[
            "so hoje",
            "apenas hoje",
            "today only",
            "nesta mensagem",
            "neste exercicio",
            "for this answer",
            "por enquanto",
        ],
    )
}
pub fn preference(span: &str) -> Option<Claim> {
    let s = fold(span);
    if !eligible_source(span)
        || span.contains('?')
        || any(
            &s,
            &[
                "talvez",
                "maybe",
                "might",
                "nao ",
                "not ",
                "don't",
                "nunca",
                "mas ",
                "but ",
                "de outra pessoa",
            ],
        )
    {
        return None;
    }
    let choose = any(
        &s,
        &[
            "prefiro",
            "prefer",
            "gosto",
            "like ",
            "favorit",
            "escolho",
            "minha escolha",
            "opto por",
        ],
    );
    let personal = any(
        &s,
        &[
            "eu ", "meu ", "minha ", "meus ", "minhas ", "prefiro ", "gosto ", "i ", "my ", "to me",
        ],
    );
    if !choose || !personal {
        return None;
    }
    let (property, value) = if any(&s, &["interface", "tema", "theme", "modo", "mode"])
        && any(&s, &["escuro", "dark"])
    {
        ("interface_theme", "dark")
    } else if any(&s, &["interface", "tema", "theme", "modo", "mode"])
        && any(&s, &["claro", "light"])
    {
        ("interface_theme", "light")
    } else if any(
        &s,
        &[
            "resposta",
            "answer",
            "reply",
            "replies",
            "comunicacao",
            "communication",
        ],
    ) && any(
        &s,
        &[
            "curta", "curto", "concisa", "conciso", "breve", "short", "concise", "brief",
        ],
    ) {
        ("communication_style", "concise")
    } else if any(
        &s,
        &[
            "resposta",
            "answer",
            "reply",
            "replies",
            "comunicacao",
            "communication",
        ],
    ) && any(&s, &["detalhad", "detailed"])
    {
        ("communication_style", "detailed")
    } else if any(
        &s,
        &[
            "idioma", "lingua", "language", "resposta", "answer", "reply", "replies",
        ],
    ) && any(&s, &["portugues", "portuguese"])
    {
        ("response_language", "pt")
    } else if any(&s, &["rpg"]) && any(&s, &["tatic", "tactic"]) {
        ("games_tactics", "positive")
    } else if any(&s, &["estrategia por turnos", "turn-based strategy"]) {
        ("games_turn_strategy", "positive")
    } else if any(&s, &["cafe", "coffee"])
        && any(
            &s,
            &[
                "sem acucar",
                "without sugar",
                "unsweetened",
                "puro, sem colocar acucar",
            ],
        )
    {
        ("coffee_sugar", "without_sugar")
    } else if any(&s, &["cafe", "coffee"]) && any(&s, &["com acucar", "with sugar"]) {
        ("coffee_sugar", "with_sugar")
    } else {
        return None;
    };
    let mut c = base(
        span,
        Kind::Preference,
        "user",
        property,
        value,
        "preference",
    );
    if property == "games_tactics" {
        c.facets.push("rpg".into());
        if s.contains("classes") {
            c.facets.push("classes".into())
        }
        if any(&s, &["evolucao", "progression", "progressao"]) {
            c.facets.push("progression".into())
        }
    }
    if transient(&s) {
        c.temporal = "temporary".into();
        c.ttl_seconds = Some(3600)
    }
    Some(c)
}
pub fn extract(text: &str, project: &str) -> Vec<Claim> {
    if !eligible_source(text) || !project_scope_matches(text, project) {
        return vec![];
    }
    let mut out = vec![];
    for raw in text.split(['.', '!', ';', '\n']) {
        let span = raw.trim();
        if span.len() < 12 || span.len() > 300 || span.contains('?') {
            continue;
        }
        let s = fold(span);
        if ambiguous_preference(&s)
            && any(
                &s,
                &[
                    "prefiro", "prefer", "gosto", "like ", "favorit", "rpg", "jogos",
                ],
            )
        {
            continue;
        }
        let mut known = crate::extraction::extract(span);
        if known.is_empty() {
            if let Some(c) = preference(span) {
                known.push(c)
            }
        }
        let subject = format!("project.{project}");
        let project_intent = any(
            &s,
            &[
                "quero usar ",
                "vou usar ",
                "decidi usar ",
                "decidi trocar ",
                "we decided to use ",
                "i decided to use ",
                "i want to use ",
                "i switched ",
            ],
        );
        if project_intent && !any(&s, &["nao ", "not ", "talvez", "maybe", "might"]) {
            let tokens = |s: &str| {
                s.split(|c: char| !c.is_alphanumeric())
                    .filter(|w| !w.is_empty())
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            };
            let target = if s.contains("trocar ") && s.contains(" por ") {
                s.rsplit(" por ").next().unwrap()
            } else {
                &s
            };
            let words = tokens(target);
            let languages = ["python", "rust", "typescript", "javascript"]
                .into_iter()
                .filter(|w| words.iter().any(|t| t == w))
                .collect::<Vec<_>>();
            let temporal = if any(&s, &["quero usar ", "i want to use ", "vou usar "]) {
                "planned"
            } else {
                "durable"
            };
            if languages.len() == 1 {
                let mut c = base(
                    span,
                    Kind::Decision,
                    &subject,
                    "language",
                    languages[0],
                    temporal,
                );
                c.project_hint = Some(project.into());
                c.content = format!(
                    "Projeto {project}: {} linguagem {}.",
                    if temporal == "planned" {
                        "planeja usar"
                    } else {
                        "decidiu usar"
                    },
                    languages[0]
                );
                known.push(c)
            }
            let dbs = ["sqlite", "postgresql", "mysql"]
                .into_iter()
                .filter(|w| words.iter().any(|t| t == w))
                .collect::<Vec<_>>();
            if dbs.len() == 1 && !known.iter().any(|c| c.property == "database") {
                let mut c = base(span, Kind::Decision, &subject, "database", dbs[0], temporal);
                c.project_hint = Some(project.into());
                c.content = format!(
                    "Projeto {project}: {} {} como banco de dados.",
                    if temporal == "planned" {
                        "planeja usar"
                    } else {
                        "decidiu usar"
                    },
                    dbs[0]
                );
                known.push(c)
            }
        }
        if known.is_empty()
            && any(
                &s,
                &[
                    "estou comecando um projeto de ",
                    "iniciei um projeto de ",
                    "i am starting a project ",
                ],
            )
        {
            let mut c = base(span, Kind::Project, &subject, "description", &s, "durable");
            c.project_hint = Some(project.into());
            known.push(c)
        }
        if known.is_empty() {
            if let Some(target) = s
                .strip_prefix("quero aprender ")
                .map(|t| format!("aprender {t}"))
                .or_else(|| {
                    s.strip_prefix("quero terminar ")
                        .map(|t| format!("terminar {t}"))
                })
            {
                let c = base(
                    span,
                    Kind::Goal,
                    "user",
                    &format!(
                        "goal.{}",
                        &digest(&format!("meu objetivo e {target}"))[..12]
                    ),
                    &s,
                    "durable",
                );
                known.push(c)
            }
        }
        if known.is_empty()
            && any(
                &s,
                &[
                    "troquei de computador",
                    "troquei meu computador principal",
                    "i replaced my computer",
                ],
            )
        {
            known.push(base(
                span,
                Kind::Hardware,
                "device.unspecified",
                "equipment_change",
                &s,
                "pending",
            ))
        }
        if known.is_empty()
            && any(
                &s,
                &[
                    "na verdade, agora sao ",
                    "na verdade agora sao ",
                    "actually, now it is ",
                ],
            )
        {
            let n = crate::coverage::amounts(&s);
            if n.len() == 1 {
                let mut c = base(
                    span,
                    Kind::Hardware,
                    "device.unspecified",
                    "capacity_update",
                    &format!("{}_gb", n[0]),
                    "pending",
                );
                c.correction = true;
                known.push(c)
            }
        }
        for mut c in known {
            c.correction |= any(
                &s,
                &[
                    "na verdade",
                    "agora prefiro",
                    "mudei de ideia",
                    "actually",
                    "now i prefer",
                    "i now prefer",
                    "decidi trocar",
                ],
            );
            if s.starts_with("quero voltar a registrar minha preferencia: ")
                || s.starts_with("i want to remember my preference again: ")
            {
                c.rule = format!("explicit_v3_renewal:{}", c.rule);
                c.confidence = 0.85
            }
            if c.kind == Kind::Preference && transient(&s) {
                c.temporal = "temporary".into();
                c.ttl_seconds = Some(3600)
            }
            if !out
                .iter()
                .any(|v: &Claim| v.fingerprint() == c.fingerprint())
            {
                out.push(c)
            }
            if out.len() == 4 {
                return out;
            }
        }
    }
    out
}
pub fn normalize(text: &str, scope: &str, project: &str, raw: &str) -> Result<Vec<Claim>> {
    if !eligible_source(text) || !project_scope_matches(text, project) || raw.len() > 12000 {
        return Err("Ineligible V3 source".into());
    }
    let output: AtomicOutput = serde_json::from_str(raw).map_err(|_| "Invalid V3 schema")?;
    if output.version != 3
        || output.scope != scope
        || output.project != project
        || output.claims.len() > 4
    {
        return Err("V3 scope/version/ceiling mismatch".into());
    }
    let mut accepted = vec![];
    for p in output.claims {
        let span = p
            .evidence
            .trim()
            .trim_end_matches(['.', '!', ';', '\n'])
            .trim();
        if !text.contains(p.evidence.trim())
            || !text.split(['.', '!', ';', '\n']).any(|s| s.trim() == span)
        {
            continue;
        }
        for mut c in extract(span, project) {
            if c.kind != p.category
                && !(p.category == Kind::Fact && matches!(c.kind, Kind::Hardware | Kind::Project))
            {
                continue;
            }
            c.confidence = c.confidence.min(0.9);
            c.relevance = c.relevance.min(0.8);
            c.rule = format!("llm_v3_grounded:{}", c.rule);
            if !accepted
                .iter()
                .any(|v: &Claim| v.fingerprint() == c.fingerprint())
            {
                accepted.push(c)
            }
        }
    }
    if accepted.len() > 4 {
        return Err("V3 interpreted claim ceiling".into());
    }
    Ok(accepted)
}
pub fn safe_preference(c: &Claim) -> bool {
    !ambiguous_preference(&fold(&c.evidence))
        && c.kind == Kind::Preference
        && c.subject == "user"
        && c.temporal == "preference"
        && c.ttl_seconds.is_none()
        && eligible_source(&c.evidence)
        && [
            "coffee_sugar",
            "response_language",
            "games_tactics",
            "games_squad_tactics",
            "games_branching_story",
            "games_turn_strategy",
            "interface_theme",
            "communication_style",
        ]
        .contains(&c.property.as_str())
        && crate::extraction::extract(&c.evidence)
            .into_iter()
            .chain(preference(&c.evidence))
            .any(|p| {
                p.kind == c.kind
                    && p.subject == c.subject
                    && p.property == c.property
                    && p.value == c.value
                    && p.temporal == c.temporal
                    && p.ttl_seconds.is_none()
                    && p.facets == c.facets
            })
}
pub fn project_summary(query: &str) -> bool {
    let q = fold(query);
    any(&q, &["projeto", "project"])
        && any(
            &q,
            &[
                "como esta",
                "como anda",
                "andamento",
                "retomar",
                "aquele projeto",
                "how is",
                "project going",
                "project progress",
            ],
        )
}
