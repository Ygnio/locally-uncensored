//! Small, auditable query relevance mechanics; no model or database authority.
use crate::extraction::fold;
pub const LEXICAL_COVERAGE: f64 = 0.4;
pub const SEMANTIC_MARGIN: f64 = 0.04;
/// Correct only one-edit domain nouns. Names, values, quantities, negation and
/// source evidence never gain fuzzy authority. Ambiguous matches abstain.
pub(crate) fn normalize_query(text: &str) -> String {
    fn close(a: &str, b: &str) -> bool {
        let a = a.chars().collect::<Vec<_>>();
        let b = b.chars().collect::<Vec<_>>();
        if a.len().abs_diff(b.len()) > 1 {
            return false;
        }
        if a.len() == b.len() {
            let differing = (0..a.len()).filter(|i| a[*i] != b[*i]).collect::<Vec<_>>();
            return differing.len() == 1
                || (differing.len() == 2
                    && differing[1] == differing[0] + 1
                    && a[differing[0]] == b[differing[1]]
                    && a[differing[1]] == b[differing[0]]);
        }
        let (short, long) = if a.len() < b.len() {
            (&a, &b)
        } else {
            (&b, &a)
        };
        let mut i = 0;
        let mut j = 0;
        let mut skipped = false;
        while i < short.len() && j < long.len() {
            if short[i] == long[j] {
                i += 1;
                j += 1
            } else if skipped {
                return false;
            } else {
                skipped = true;
                j += 1
            }
        }
        true
    }
    const NOUNS: &[&str] = &[
        "linguagem",
        "language",
        "computador",
        "computer",
        "notebook",
        "memoria",
        "memory",
        "preferencia",
        "preference",
        "coffee",
        "acucar",
        "database",
    ];
    let text = fold(text);
    text.split_inclusive(|c: char| !c.is_alphanumeric())
        .map(|part| {
            let word = part.trim_end_matches(|c: char| !c.is_alphanumeric());
            let matches = if word.len() >= 5 && !NOUNS.contains(&word) {
                NOUNS
                    .iter()
                    .filter(|known| close(word, known))
                    .copied()
                    .collect::<Vec<_>>()
            } else {
                vec![]
            };
            format!(
                "{}{}",
                if matches.len() == 1 { matches[0] } else { word },
                &part[word.len()..]
            )
        })
        .collect()
}
pub fn tokens(text: &str) -> Vec<String> {
    let folded = normalize_query(text);
    let mut words = Vec::new();
    for raw in folded.split(|c: char| !c.is_alphanumeric()) {
        if raw.len() < 2 || STOP.contains(&raw) {
            continue;
        }
        let word = match raw {
            "computador" | "computer" | "pc" => "computer",
            "notebook" | "laptop" | "note" => "laptop",
            "memoria" | "memory" | "ram" => "ram",
            "trabalho" | "work" | "office" | "empresa" => "work",
            "principal" | "primary" | "main" => "primary",
            "outro" | "other" | "secondary" | "secundario" => "other",
            "cafe" | "coffee" | "espresso" => "coffee",
            "acucar" | "sugar" | "unsweetened" => "sugar",
            "jogo" | "jogos" | "game" | "games" | "rpg" | "rpgs" => "games",
            "personagem" | "personagens" | "character" | "characters" => "character",
            "classes" | "class" | "profissoes" | "professions" => "classes",
            "objetivo" | "objetivos" | "goal" | "goals" => "goal",
            "tarefa" | "task" => "task",
            "installed" | "instalada" | "instalados" | "instalado" => "installed",
            "livre" | "livres" | "free" | "available" | "disponivel" => "free",
            _ => raw,
        };
        if let Some(n) = word
            .strip_suffix("gb")
            .filter(|v| v.chars().all(|c| c.is_ascii_digit()) && !v.is_empty())
        {
            words.push(n.into());
            words.push("gb".into());
        } else {
            words.push(word.into())
        }
    }
    words.sort();
    words.dedup();
    words
}
const STOP: &[&str] = &[
    "eu",
    "meu",
    "minha",
    "meus",
    "minhas",
    "voce",
    "que",
    "qual",
    "quais",
    "como",
    "para",
    "por",
    "uma",
    "um",
    "de",
    "da",
    "do",
    "das",
    "dos",
    "em",
    "no",
    "na",
    "nos",
    "nas",
    "se",
    "com",
    "sem",
    "the",
    "what",
    "which",
    "how",
    "is",
    "are",
    "do",
    "does",
    "my",
    "your",
    "and",
    "of",
    "to",
    "for",
    "about",
    "tell",
    "recall",
    "remember",
    "lembra",
    "lembre",
    "sobre",
    "memoria",
    "memorias",
    "please",
    "favor",
    "obrigado",
    "thanks",
    "in",
    "on",
    "at",
    "our",
    "has",
    "have",
    "am",
    "an",
    "it",
    "its",
    "would",
    "should",
    "much",
    "many",
    "tem",
    "possui",
    "esta",
    "e",
    "a",
    "o",
    "as",
    "os",
    "with",
    "by",
    "or",
    "not",
    "drop",
    "table",
    "memories",
    "recomende",
    "recommend",
    "suggest",
    "gostos",
    "tenho",
    "quanta",
    "quanto",
    "quantos",
    "agora",
    "now",
    "nesta",
];
pub fn lexical_coverage(query: &str, content: &str) -> f64 {
    let q = tokens(query);
    let d = tokens(content);
    if q.is_empty() {
        return 0.;
    }
    q.iter().filter(|v| d.contains(v)).count() as f64 / q.len() as f64
}
pub fn personal(query: &str) -> bool {
    let padded = format!(" {} ", fold(query));
    [
        " my ",
        " i ",
        " me ",
        " meu ",
        " minha ",
        " meus ",
        " minhas ",
        " eu ",
        " para mim ",
        "pref",
        "gosto",
        "recom",
        "suggest",
        "sugir",
        "lembra",
    ]
    .iter()
    .any(|w| padded.contains(w))
}
pub fn preference_intent(query: &str) -> bool {
    if personal(query) {
        return true;
    }
    let q = fold(query);
    let games = ["jogo", "game", "rpg"].iter().any(|w| q.contains(w));
    (games
        && ["which ", "quais ", "indique", "opcoes", "options"]
            .iter()
            .any(|w| q.contains(w)))
        || (!q.contains('?')
            && tokens(&q).len() <= 8
            && ["cafe", "coffee", "rpg", "jogos", "games", "portugues"]
                .iter()
                .any(|w| q.contains(w)))
}
pub fn knowledge_only(query: &str) -> bool {
    let q = fold(query);
    !personal(&q)
        && ![
            "principal",
            "primary computer",
            "work laptop",
            "office laptop",
            "office notebook",
            "main desktop",
            "secondary computer",
            "notebook do trabalho",
            "projeto ",
            "project ",
        ]
        .iter()
        .any(|w| q.contains(w))
        && ([
            "what is ",
            "define ",
            "explain ",
            "explique ",
            "o que e ",
            "como funciona ",
            "how does ",
            "how do ",
        ]
        .iter()
        .any(|w| q.starts_with(w))
            || q.contains("capital of ")
            || q.contains("capital da ")
            || q.contains("capital de "))
}
