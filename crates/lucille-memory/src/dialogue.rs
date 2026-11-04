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
    if !(12..=1600).contains(&text.len()) {
        return false;
    }
    let folded = fold(text);
    let header =
        folded.trim_start_matches(|c: char| c.is_whitespace() || ['*', '_', '[', ']'].contains(&c));
    if [
        "assistant",
        "assistente",
        "tool",
        "ferramenta",
        "user",
        "usuario",
    ]
    .iter()
    .any(|tag| {
        header
            .strip_prefix(tag)
            .is_some_and(|rest| rest.trim_start().starts_with([':', '：']))
    }) {
        return false;
    }

    if any(&folded, &["nao ", "not ", "never ", "nunca "])
        && any(
            &folded,
            &[
                "afirm", "declar", "confirm", "claim", "saying", "said", "disse", "digo",
            ],
        )
    {
        return false;
    }

    // Fictional speech frames taint an embedded personal claim. A game's
    // character progression is an object of a real user's preference, not an author.
    let personal_at = [
        "eu prefiro",
        "eu gosto",
        "eu curto",
        "i prefer",
        "i like",
        "i don't like",
        "i do not like",
        "prefiro ",
    ]
    .iter()
    .filter_map(|v| folded.find(v))
    .min()
    .unwrap_or(folded.len());
    if let Some(actor_at) = ["personag", "character"]
        .iter()
        .filter_map(|v| folded.find(v))
        .min()
    {
        if actor_at < personal_at {
            let frame = &folded[..personal_at];
            if frame[actor_at..].contains(':')
                || any(
                    frame,
                    &[
                        "falou",
                        "afirm",
                        "declar",
                        "disse",
                        "says",
                        "said",
                        "stated",
                        "no papel",
                        "in the role",
                        "como personagem",
                        "as a character",
                    ],
                )
            {
                return false;
            }
        }
    }
    let chars = text.chars().collect::<Vec<_>>();
    let quoted = text.contains(['“', '‘', '«', '「'])
        || chars.windows(2).enumerate().any(|(i, pair)| {
            pair[0] == '\''
                && pair[1].is_alphabetic()
                && (i == 0 || !chars[i - 1].is_alphanumeric())
        });
    if quoted
        || any(
            &folded,
            &[
                "outra pessoa",
                "another person",
                "someone else",
                "not mine",
                "nao e minha",
                "nao e meu",
            ],
        )
    {
        return false;
    }

    // Source framing and denial of authorship dominate any embedded first-person
    // preference. A proposal cannot strip these qualifiers to gain authority.
    let tokens = folded
        .split(|c: char| !c.is_alphanumeric())
        .collect::<Vec<_>>();
    if tokens.iter().any(|w| {
        w.starts_with("fictic")
            || w.starts_with("ficc")
            || w.starts_with("fiction")
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
                "nunca",
                "never",
                "nem",
                "mentira",
                "falso",
                "falsehood",
                "false",
                "falsa",
                "falsidade",
                "untrue",
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
            "projeto",
            "project",
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
            "favourite",
            "preferred",
            "respostas",
            "answers",
            "replies",
            "tema",
            "theme",
            "comunicacao",
            "bebida",
            "drink",
            "beverage",
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
    let quoted = s.split_whitespace().collect::<Vec<_>>();
    if quoted.windows(2).any(|pair| {
        ["projeto", "project"].contains(&pair[0])
            && pair[1].starts_with(['"', '\'', '`', '“', '‘', '«'])
            && pair[1].trim_matches(|c: char| !c.is_alphanumeric()) != fold(project)
    }) {
        return false;
    }
    // Closed-class temporal/deictic phrases are not identifiers. Preserve an
    // explicit proper-case/quoted name even when its spelling is a connective.
    let original = text.split_whitespace().collect::<Vec<_>>();
    if original.windows(2).any(|p| {
        ["project", "projeto"].contains(&fold(p[0]).as_str())
            && ["hoje", "today", "continua", "continues", "neste", "nesse"]
                .contains(&fold(p[1]).as_str())
            && p[1].chars().next().is_some_and(|c| c.is_uppercase())
            && fold(p[1]) != fold(project)
    }) {
        return false;
    }
    let w = s
        .split_whitespace()
        .map(|s| s.trim_matches(|c: char| !c.is_alphanumeric()))
        .collect::<Vec<_>>();
    if w.len() > 1
        && ["projeto", "project"].contains(&w[w.len() - 2])
        && [
            "em", "in", "que", "that", "which", "de", "do", "da", "com", "sem", "para", "with",
            "without", "about", "sobre",
        ]
        .contains(&w[w.len() - 1])
        && w[w.len() - 1] != fold(project)
    {
        return false;
    }
    !w.windows(2).enumerate().any(|(index, p)| {
        ["project", "projeto"].contains(&p[0])
            && !p[1].is_empty()
            && !(["neste", "nesse"].contains(&p[1])
                && w.get(index + 2)
                    .is_some_and(|word| ["momento", "estado", "ponto", "contexto"].contains(word)))
            && ![
                "de",
                "do",
                "da",
                "novo",
                "new",
                "pessoal",
                "personal",
                "is",
                "e",
                "usa",
                "uses",
                "utiliza",
                "requires",
                "requer",
                "precisa",
                "needs",
                "aqui",
                "here",
                "going",
                "agora",
                "now",
                "eu",
                "i",
                "atualmente",
                "currently",
                "esta",
                "estava",
                "estara",
                "foi",
                "sera",
                "vai",
                "nao",
                "ja",
                "ainda",
                "atual",
                "current",
                "was",
                "will",
                "database",
                "banco",
                "dados",
                "stores",
                "should",
                "has",
                "tem",
                "ficticio",
                "configuration",
                "doing",
                "que",
                "sobre",
                "para",
                "sem",
                "com",
                "that",
                "which",
                "about",
                "em",
                "in",
                "pretendo",
                "pretende",
                "planejo",
                "planeja",
                "hoje",
                "today",
                "continua",
                "continues",
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
    if s.split(|c: char| !c.is_alphanumeric())
        .any(|w| ["hoje", "today"].contains(&w))
    {
        return true;
    }
    any(
        s,
        &[
            "so hoje",
            "apenas hoje",
            "today only",
            "only today",
            "just today",
            "somente hoje",
            "for now",
            "for this exercise",
            "nesta resposta",
            "this time",
            "nesta mensagem",
            "neste exercicio",
            "for this answer",
            "por enquanto",
        ],
    )
}
fn preference_authorship(span: &str) -> bool {
    let s = format!(" {} ", fold(span));
    any(
        &s,
        &[
            " prefiro ",
            " gosto ",
            " adoro ",
            " curto ",
            " detesto ",
            " odeio ",
            " opto por ",
            " eu amo ",
            " i prefer ",
            " i like ",
            " i love ",
            " i adore ",
            " i enjoy ",
            " i dislike ",
            " i don't like ",
            " i do not like ",
            " meu idioma preferido ",
            " minha lingua preferida ",
            " minha preferencia de interface ",
            " minha preferencia de tema ",
            " minha preferencia de comunicacao ",
            " minha preferencia e ",
            " minha bebida favorita ",
            " minha bebida preferida ",
            " minhas preferidas ",
            " meus preferidos ",
            " me agrada ",
            " me incomodam ",
            " mais aprecio ",
            " my preferred language ",
            " my favorite drink ",
            " my favourite drink ",
            " my favorite beverage ",
            " my favourite beverage ",
            " my preferred theme ",
            " my favorite hobby ",
            " my favourite hobby ",
            " sao meus favoritos ",
            " sao minhas favoritas ",
            " are my favorites ",
            " are my favourites ",
            " are my favorite ",
            " are my favourite ",
            " my favorite games ",
            " my favourite games ",
        ],
    )
}
pub fn preference(span: &str) -> Option<Claim> {
    let s = fold(span);
    if !preference_authorship(span) {
        return None;
    }

    if eligible_source(span)
        && !span.contains('?')
        && !ambiguous_preference(&s)
        && any(
            &s,
            &[
                "nao curto",
                "nao gosto",
                "nao prefiro",
                "i dislike",
                "i don't like",
                "i do not like",
            ],
        )
        && any(&s, &["interface", "screen", "tela"])
        && any(&s, &["carregad", "poluid", "clutter"])
        && !any(&s, &[" mas ", " but ", "talvez", "maybe"])
    {
        let mut c = base(
            span,
            Kind::Preference,
            "user",
            "interface_clutter",
            "negative",
            "preference",
        );
        if transient(&s) {
            c.temporal = "temporary".into();
            c.ttl_seconds = Some(3600)
        }
        return Some(c);
    }

    if eligible_source(span)
        && !span.contains('?')
        && !ambiguous_preference(&s)
        && !any(&s, &[" but ", " mas ", "maybe", "talvez"])
        && any(
            &s,
            &[
                "i don't like tactical rpg",
                "i do not like tactical rpg",
                "i dislike tactical rpg",
            ],
        )
    {
        let mut c = base(
            span,
            Kind::Preference,
            "user",
            "games_tactics",
            "negative",
            "preference",
        );
        c.facets.push("rpg".into());
        if s.contains("classes") {
            c.facets.push("classes".into());
        }
        if s.contains("progression") {
            c.facets.push("progression".into());
        }
        c.content = crate::extraction::preference_content("negative", &c.facets);
        if transient(&s) {
            c.temporal = "temporary".into();
            c.ttl_seconds = Some(3600);
        }
        return Some(c);
    }

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
            "adoro",
            "adore",
            "love",
            "enjoy",
            "curto",
            "favourit",
            "amo ",
            "like ",
            "favorit",
            "escolho",
            "minha escolha",
            "detesto",
            "desgosto",
            "dislike",
            "odeio",
            "opto por",
            "preferida",
            "preferido",
            "me agrada",
            "me incomodam",
            "mais aprecio",
        ],
    );
    let personal = any(
        &s,
        &[
            "eu ",
            "meu ",
            "minha ",
            "meus ",
            "minhas ",
            "prefiro ",
            "gosto ",
            "adoro ",
            "amo ",
            "i ",
            "my ",
            "to me",
            "me agrada",
            "me incomodam",
            "aprecio",
        ],
    );
    if !choose || !(personal || preference_authorship(span)) {
        return None;
    }
    let (property, value) = if any(&s, &["jogo", "game", "rpg"])
        && any(
            &s,
            &[
                "criacao de personagens",
                "criar personagens",
                "character creation",
                "creating characters",
            ],
        ) {
        ("games_character_creation", "positive")
    } else if any(&s, &["jogo", "game", "rpg"])
        && (any(
            &s,
            &[
                "escolh",
                "choos",
                "class selection",
                "definir",
                "selecionar",
            ],
        ) && any(&s, &["classe", "class"]))
    {
        ("games_class_selection", "positive")
    } else if any(&s, &["interface", "tela", "screen"])
        && any(&s, &["poluida", "clutter", "carregada"])
        && any(
            &s,
            &["desgosto", "detesto", "dislike", "odeio", "me incomodam"],
        )
    {
        ("interface_clutter", "negative")
    } else if any(
        &s,
        &[
            "bebida favorita",
            "favorite drink",
            "bebida preferida",
            "favourite drink",
            "bebidas",
        ],
    ) && s
        .split(|c: char| !c.is_alphanumeric())
        .any(|w| w == "cha" || w == "tea")
    {
        ("favorite_beverage", "tea")
    } else if any(&s, &["resposta", "answer", "reply", "replies"])
        && any(&s, &["objetiv", "objective"])
    {
        ("communication_focus", "objective")
    } else if any(&s, &["violao acustico", "acoustic guitar"]) && any(&s, &["tocar", "play"]) {
        ("hobby", "acoustic_guitar")
    } else if any(&s, &["violao acustico", "acoustic guitar"]) {
        ("music_instrument", "acoustic_guitar")
    } else if any(&s, &["romances de aventura", "adventure novels"]) {
        ("book_genre", "adventure")
    } else if any(&s, &["trilhas curtas", "short trails"]) {
        ("outing", "short_trails")
    } else if any(&s, &["foto", "photo"])
        && any(
            &s,
            &[
                "papel brilhante",
                "fotos brilhantes",
                "impressao brilhante",
                "glossy photo prints",
                "glossy photo paper",
            ],
        )
    {
        ("photo_paper", "glossy")
    } else if any(
        &s,
        &[
            "estantes de madeira",
            "wooden bookshelves",
            "wood bookshelves",
        ],
    ) {
        ("shelf_material", "wood")
    } else if any(&s, &["interface", "tema", "theme", "modo", "mode"])
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
            "explicac",
            "explanation",
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
            "explicac",
            "explanation",
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
fn additional_preferences(span: &str) -> Vec<Claim> {
    let Some(primary) = preference(span) else {
        return vec![];
    };
    let s = fold(span);
    if primary.property == "games_character_creation"
        && any(&s, &["escolh", "choos", "class selection"])
        && any(&s, &["classe", "class"])
    {
        let mut secondary = primary;
        secondary.property = "games_class_selection".into();
        vec![secondary]
    } else {
        vec![]
    }
}
/// Project choice and observed implementation are separate tracks. A future
/// language choice cannot erase the language in an existing implementation.
pub(crate) fn execution_track(subject: &str, property: &str, temporal: &str) -> bool {
    subject.starts_with("project.")
        && ["language", "database"].contains(&property)
        && ["in_progress", "current", "completed", "cancelled"].contains(&temporal)
}
pub(crate) fn separate_tracks(subject: &str, property: &str, left: &str, right: &str) -> bool {
    subject.starts_with("project.")
        && ["language", "database"].contains(&property)
        && execution_track(subject, property, left) != execution_track(subject, property, right)
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
        for c in preference(span)
            .into_iter()
            .chain(additional_preferences(span))
        {
            if let Some(old) = known
                .iter_mut()
                .find(|old| old.subject == c.subject && old.property == c.property)
            {
                if old.value == c.value
                    && old.temporal == c.temporal
                    && c.facets.len() > old.facets.len()
                    && old.facets.iter().all(|f| c.facets.contains(f))
                {
                    *old = c;
                }
            } else {
                known.push(c)
            }
        }
        let subject = format!("project.{project}");
        if any(&s, &["talvez eu use ", "maybe i will use ", "i might use "])
            && any(&s, &["projeto", "project"])
            && !any(&s, &["nao ", "not "])
        {
            for (property, values) in [
                (
                    "language",
                    &["python", "rust", "typescript", "javascript"][..],
                ),
                ("database", &["sqlite", "postgresql", "mysql"][..]),
            ] {
                let found = values
                    .iter()
                    .filter(|value| {
                        s.split(|c: char| !c.is_alphanumeric())
                            .any(|w| w == **value)
                    })
                    .collect::<Vec<_>>();
                if found.len() == 1 {
                    known.retain(|c| c.property != "hardware_intention" && c.property != property);
                    let mut c = base(
                        span,
                        Kind::Hypothesis,
                        &subject,
                        property,
                        found[0],
                        "hypothesis",
                    );
                    c.project_hint = Some(project.into());
                    c.ttl_seconds = Some(7 * 86400);
                    known.push(c);
                }
            }
        }
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
                "pretendo usar ",
                "i plan to use ",
                "meus planos para o projeto",
                "escolhi ",
                "i chose ",
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
            let temporal = if any(
                &s,
                &[
                    "quero usar ",
                    "i want to use ",
                    "vou usar ",
                    "pretendo usar ",
                    "i plan to use ",
                    "meus planos para o projeto",
                ],
            ) {
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
        if any(
            &s,
            &[
                "ainda nao decidi",
                "ainda nao escolhi",
                "i have not decided",
                "i have not chosen",
                "ainda nao defini",
            ],
        ) {
            for (property, labels) in [
                ("language", &["linguagem", "language"][..]),
                ("database", &["banco de dados", "database"][..]),
            ] {
                if any(&s, labels) {
                    known.retain(|c| c.property != property);
                    let mut c = base(
                        span,
                        Kind::Decision,
                        &subject,
                        property,
                        "undecided",
                        "unknown",
                    );
                    c.project_hint = Some(project.into());
                    known.push(c);
                }
            }
        }
        for (temporal, phrases) in [
            (
                "completed",
                &[
                    "conclui meu projeto",
                    "conclui o meu projeto",
                    "i completed my project",
                ][..],
            ),
            (
                "cancelled",
                &[
                    "cancelei meu projeto",
                    "cancelei o meu projeto",
                    "i cancelled my project",
                ][..],
            ),
        ] {
            if any(&s, phrases) {
                let mut c = base(
                    span,
                    Kind::Project,
                    &subject,
                    "lifecycle",
                    temporal,
                    temporal,
                );
                c.project_hint = Some(project.into());
                known.push(c);
            }
        }
        let observed = if any(
            &s,
            &[
                "conclui a implementacao",
                "terminei a implementacao",
                "i completed the implementation",
            ],
        ) {
            Some("completed")
        } else if any(
            &s,
            &["cancelei a implementacao", "i cancelled the implementation"],
        ) {
            Some("cancelled")
        } else if any(
            &s,
            &[
                "estou desenvolvendo",
                "estou implementando",
                "i am developing",
                "i am implementing",
            ],
        ) {
            Some("in_progress")
        } else if any(
            &s,
            &[
                "meu projeto usa ",
                "my project uses ",
                "no meu projeto, eu uso ",
                "no meu projeto eu uso ",
                "in my project, i use ",
                "in my project i use ",
            ],
        ) {
            Some("current")
        } else {
            None
        };
        if let Some(temporal) = observed.filter(|_| {
            !any(
                &s,
                &[
                    "nao ", "not ", "talvez", "maybe", "might", "se eu ", "if i ",
                ],
            )
        }) {
            for (property, values) in [
                (
                    "language",
                    &["python", "rust", "typescript", "javascript"][..],
                ),
                ("database", &["sqlite", "postgresql", "mysql"][..]),
            ] {
                let found = values
                    .iter()
                    .filter(|v| s.split(|c: char| !c.is_alphanumeric()).any(|w| w == **v))
                    .collect::<Vec<_>>();
                if found.len() == 1 {
                    known.retain(|c| c.property != property);
                    let mut c = base(span, Kind::Fact, &subject, property, found[0], temporal);
                    c.project_hint = Some(project.into());
                    known.push(c);
                }
            }
        }
        if known.is_empty()
            && any(
                &s,
                &[
                    "estou comecando um projeto de ",
                    "iniciei um projeto de ",
                    "i am starting a project ",
                    "o projeto continua sendo um ",
                    "meu projeto continua sendo um ",
                    "my project remains a ",
                    "the project remains a ",
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
            if c.kind == Kind::Preference && !preference_authorship(&c.evidence) {
                continue;
            }
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
                || s.starts_with("pode voltar a memorizar: ")
                || s.starts_with("you can remember this again: ")
            {
                c.rule = format!("explicit_v3_renewal:{}", c.rule);
                c.confidence = 0.85
            }
            if c.kind == Kind::Preference && transient(&s) {
                c.temporal = "temporary".into();
                c.ttl_seconds = Some(3600)
            }
            if !out.iter().any(|v: &Claim| {
                v.fingerprint() == c.fingerprint()
                    && v.temporal == c.temporal
                    && v.facets == c.facets
            }) {
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
            // Coarse B-to-B taxonomy hints never gain promotion authority. The
            // complete literal clause is independently parsed into the canonical kind.
            let review_alias = (p.category == Kind::Goal
                && matches!(c.kind, Kind::Task | Kind::Solution))
                || (p.category == Kind::Decision && c.kind == Kind::Constraint)
                || (p.category == Kind::Project
                    && c.kind == Kind::Hardware
                    && c.property == "equipment_change");
            if c.kind != p.category
                && !(p.category == Kind::Fact && matches!(c.kind, Kind::Hardware | Kind::Project))
                && !review_alias
            {
                continue;
            }
            c.confidence = c.confidence.min(0.9);
            c.relevance = c.relevance.min(0.8);
            c.rule = format!("llm_v3_grounded:{}", c.rule);
            if !accepted.iter().any(|v: &Claim| {
                v.fingerprint() == c.fingerprint()
                    && v.temporal == c.temporal
                    && v.facets == c.facets
            }) {
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
        && preference_authorship(&c.evidence)
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
            "games_character_creation",
            "games_class_selection",
            "interface_clutter",
            "favorite_beverage",
            "communication_focus",
            "music_instrument",
            "book_genre",
            "hobby",
            "outing",
            "photo_paper",
            "shelf_material",
        ]
        .contains(&c.property.as_str())
        && crate::extraction::extract(&c.evidence)
            .into_iter()
            .chain(preference(&c.evidence))
            .chain(additional_preferences(&c.evidence))
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
        && !any(
            &q,
            &[
                "banco",
                "database",
                "linguagem",
                "language",
                "goal",
                "objetivo",
                "meta conclu",
            ],
        )
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
                "lembra",
                "recorda",
                "remember",
                "recall",
            ],
        )
}
