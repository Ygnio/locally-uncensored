//! Deliberately bounded rule baseline. Unsupported statements abstain; no model is loaded.
use crate::Kind;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
pub fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ã' | 'â' | 'ä' => 'a',
            'é' | 'ê' | 'è' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ô' | 'õ' | 'ö' | 'ò' => 'o',
            'ú' | 'ü' | 'ù' | 'û' => 'u',
            'ç' => 'c',
            '’' | '‘' => '\'',
            _ => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub kind: Kind,
    pub content: String,
    pub subject: String,
    pub property: String,
    pub value: String,
    pub confidence: f64,
    pub relevance: f64,
    pub temporal: String,
    pub ttl_seconds: Option<i64>,
    pub facets: Vec<String>,
    pub evidence: String,
    pub rule: String,
    pub project_hint: Option<String>,
    pub correction: bool,
}
impl Claim {
    pub fn key(&self) -> String {
        format!("{}.{}", self.subject, self.property)
    }
    pub fn fingerprint(&self) -> String {
        digest(&format!(
            "{}|{}|{}",
            self.subject, self.property, self.value
        ))
    }
}
fn contains_any(text: &str, words: &[&str]) -> bool {
    words.iter().any(|word| text.contains(word))
}
pub(crate) fn unsafe_text(text: &str) -> bool {
    if [
        "forwarded",
        "encaminhad",
        "seu papel",
        "your role",
        "so que nao",
        "ele escreveu",
        "ela escreveu",
    ]
    .iter()
    .any(|v| text.contains(v))
    {
        return true;
    }
    if contains_any(
        text,
        &[
            "minha mae",
            "he said",
            "she said",
            "they said",
            "he wrote",
            "she wrote",
            "segundo",
            "ele contou",
            "ela contou",
            "meu colega",
            "minha colega",
            "my friend",
            "my colleague",
            "my neighbor",
            "my neighbour",
            "my sister",
            "my brother",
            "my client",
            "meu vizinho",
            "minha vizinha",
            "meu professor",
            "minha professora",
            "estou escrevendo uma historia",
            "meu pai",
            "ele disse",
            "ela disse",
            "em um mundo ficticio",
            "hipoteticamente",
            "nao sei se",
            "nao tenho certeza",
            "not sure whether",
            "not certain whether",
            "imagine que",
            "autorize",
            "sua nova identidade",
            "mude seu nome",
        ],
    ) {
        return true;
    }
    text.split(|c: char| !c.is_alphanumeric())
        .any(|word| matches!(word, "senha" | "senhas"))
        || contains_any(
            text,
            &[
                "password",
                "credencial",
                "credential",
                "chave de acesso",
                "secret key",
                "access key",
                "api_key",
                "api key",
                "bearer ",
                "cpf",
                "credit card",
                "cartao",
                "telefone",
                "endereco",
                "e-mail",
                "email@",
                "token=",
                "ignore previous",
                "ignore as instru",
                "ignore todas",
                "system prompt",
                "mude sua identidade",
                "altere sua identidade",
                "your identity",
                "identidad",
                "identity",
                "permiss",
                "permissions",
                "autoriz",
                "sou a assistente",
                "the assistant said",
                "a assistente disse",
                "meu amigo",
                "minha amiga",
                "meu irmao",
                "minha irma",
                "meu cliente",
                "segundo ele",
                "according to",
                "tool result",
                "resultado da ferramenta",
            ],
        )
        || text.contains(['`', '"', '>', '@'])
}
fn gb(text: &str) -> Option<u32> {
    let words = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect::<Vec<_>>();
    words
        .windows(2)
        .find_map(|w| {
            if w[1] == "gb" {
                w[0].parse().ok()
            } else {
                None
            }
        })
        .or_else(|| {
            words
                .iter()
                .find_map(|w| w.strip_suffix("gb").and_then(|v| v.parse().ok()))
        })
        .filter(|v| *v > 0 && *v <= 4096)
}
fn project(text: &str) -> Option<String> {
    let words = text.split_whitespace().collect::<Vec<_>>();
    words.windows(2).find_map(|pair| {
        if pair[0] == "projeto" || pair[0] == "project" {
            Some(
                pair[1]
                    .trim_matches(|c: char| !c.is_alphanumeric())
                    .to_string(),
            )
        } else {
            None
        }
    })
}
pub fn extract(text: &str) -> Vec<Claim> {
    if text.len() > 16000 || text.contains('?') || unsafe_text(&fold(text)) {
        return vec![];
    }
    text.split(['.', '!', ';', '\n'])
        .filter_map(classify)
        .take(4)
        .collect()
}
fn classify(sentence: &str) -> Option<Claim> {
    let evidence = sentence.trim();
    if evidence.chars().count() < 12 || evidence.len() > 300 {
        return None;
    }
    let s = fold(evidence);
    if contains_any(&s, &[" mas ", " but ", " porem "]) {
        return None;
    }
    if unsafe_text(&s) {
        return None;
    }
    if !crate::coverage::amounts(&s).is_empty() && crate::coverage::mixed_devices(&s) {
        return None;
    }
    if let Some(c) = crate::coverage::supplement(evidence) {
        return Some(c);
    }
    if crate::coverage::amounts(&s).len() > 1 {
        return None;
    }
    // Negated coffee/language statements need an explicit interpretation that
    // this bounded extractor does not provide. Abstain instead of reversing it.
    if contains_any(&s, &["cafe", "coffee", "portugues", "portuguese"])
        && contains_any(
            &s,
            &[
                "nao gosto",
                "nao prefiro",
                "nao curto",
                "don't like",
                "do not like",
                "don't prefer",
                "do not prefer",
            ],
        )
    {
        return None;
    }
    if unsafe_text(&s) {
        return None;
    }
    let mut claim = Claim {
        kind: Kind::Fact,
        content: evidence.to_string(),
        subject: "user".into(),
        property: String::new(),
        value: String::new(),
        confidence: 0.98,
        relevance: 0.9,
        temporal: "durable".into(),
        ttl_seconds: None,
        facets: vec![],
        evidence: evidence.to_string(),
        rule: String::new(),
        project_hint: None,
        correction: contains_any(
            &s,
            &[
                "agora instalei",
                "corrigindo",
                "na verdade",
                "nao gosto mais",
                "actually",
                "upgraded",
                "now installed",
                "correcao",
            ],
        ),
    };
    let personal = contains_any(
        &s,
        &[
            "meu ",
            "minha ",
            "eu ",
            "prefiro ",
            "gosto ",
            "curto ",
            "instalei ",
            "tenho ",
            "estou ",
            "preciso ",
            "resolvi ",
            "nao posso ",
            "my ",
            "i ",
            "we ",
            "nosso ",
            "decidimos ",
        ],
    );
    if !personal
        && !s.starts_with("o projeto ")
        && !s.starts_with("the project ")
        && !s.starts_with("the fictional project ")
    {
        return None;
    }
    if contains_any(
        &s,
        &[
            "talvez",
            "quem sabe",
            "pretendo comprar",
            "maybe",
            "might buy",
        ],
    ) {
        claim.kind = Kind::Hypothesis;
        claim.property = "hardware_intention".into();
        claim.value = s.clone();
        claim.temporal = "hypothesis".into();
        claim.ttl_seconds = Some(7 * 86400);
        claim.confidence = 0.8;
        claim.relevance = 0.65;
        claim.rule = "explicit_hypothesis".into();
    } else if let Some(amount) = gb(&s).filter(|_| contains_any(&s, &["ram", "memoria", "memory"]))
    {
        if !contains_any(
            &s,
            &[
                "meu pc",
                "meu computador",
                "meu outro computador",
                "meu notebook de trabalho",
                "my computer",
                "my pc",
                "my other computer",
                "my work laptop",
                "instalei ",
                "i installed ",
                "i upgraded ",
                "estou com ",
                "no computador principal",
                "on my primary computer",
            ],
        ) {
            return None;
        }
        if contains_any(
            &s,
            &[
                "nao tem",
                "nao tenho",
                "nao possui",
                "does not have",
                "doesn't have",
                "not installed",
            ],
        ) {
            return None;
        }
        claim.subject = if contains_any(&s, &["outro computador", "other computer"]) {
            "device.other_computer"
        } else if contains_any(&s, &["notebook de trabalho", "work laptop"]) {
            "device.work_laptop"
        } else {
            "device.primary_computer"
        }
        .into();
        if contains_any(&s, &["livre", "disponive", "free now", "available now"]) {
            claim.kind = Kind::Fact;
            claim.property = "free_ram".into();
            claim.temporal = "temporary".into();
            claim.ttl_seconds = Some(3600);
            claim.relevance = 0.5;
            claim.rule = "temporary_ram_observation".into();
        } else if contains_any(
            &s,
            &[
                " tem ",
                " possui ",
                " instalei ",
                " tenho ",
                " has ",
                " installed ",
                " upgraded ",
                "meu pc tem ",
            ],
        ) {
            claim.kind = Kind::Hardware;
            claim.property = "ram_capacity".into();
            claim.rule = "explicit_device_ram".into();
        } else {
            return None;
        }
        claim.value = format!("{amount}_gb");
        let device = match claim.subject.as_str() {
            "device.other_computer" => "O outro computador",
            "device.work_laptop" => "O notebook de trabalho",
            _ => "O computador principal",
        };
        claim.content = format!(
            "{device}: {amount} GB de RAM{}.",
            if claim.temporal == "temporary" {
                " livres nesta observação temporária"
            } else {
                " instalados"
            }
        );
    } else if contains_any(&s, &["cafe", "coffee"])
        && contains_any(&s, &["gosto", "prefiro", "curto", "i prefer", "i like"])
    {
        claim.kind = Kind::Preference;
        claim.property = "coffee_sugar".into();
        claim.value = if contains_any(&s, &["sem acucar", "without sugar", "unsweetened"]) {
            "without_sugar"
        } else if contains_any(&s, &["com acucar", "with sugar"]) {
            "with_sugar"
        } else {
            return None;
        }
        .into();
        claim.content = if claim.value == "without_sugar" {
            "Prefere café sem açúcar."
        } else {
            "Prefere café com açúcar."
        }
        .into();
        claim.temporal = "preference".into();
        claim.rule = "explicit_coffee_preference".into();
    } else if contains_any(&s, &["gosto", "prefiro", "curto", "i like", "i prefer"])
        && (contains_any(&s, &["rpg tatico", "rpgs taticos", "tactical rpg"])
            || (contains_any(&s, &["turnos", "turn-based"])
                && contains_any(&s, &["classes", "class-based"])))
    {
        claim.kind = Kind::Preference;
        claim.property = "games_tactics".into();
        claim.value = if contains_any(&s, &["nao gosto", "nao curto", "dislike", "don't like"]) {
            "negative"
        } else {
            "positive"
        }
        .into();
        if s.contains("rpg") {
            claim.facets.push("rpg".into())
        }
        if s.contains("classes") || s.contains("class-based") {
            claim.facets.push("classes".into())
        }
        if contains_any(&s, &["evolucao", "progressao", "progression"]) {
            claim.facets.push("progression".into())
        }
        claim.content = preference_content(&claim.value, &claim.facets);
        claim.temporal = "preference".into();
        claim.rule = "explicit_tactical_preference".into();
    } else if contains_any(&s, &["prefiro", "i prefer"])
        && contains_any(&s, &["portugues", "portuguese"])
    {
        claim.kind = Kind::Preference;
        claim.property = "response_language".into();
        claim.value = "pt".into();
        claim.content = "Prefere respostas em português.".into();
        claim.temporal = "preference".into();
        claim.rule = "explicit_language_preference".into();
    } else if let Some(p) =
        project(&s).filter(|_| contains_any(&s, &["sqlite", "postgresql", "mysql"]))
    {
        if !contains_any(
            &s,
            &[
                " utiliza ",
                " usa ",
                " uses ",
                "decidimos usar",
                "decided to use",
            ],
        ) {
            return None;
        }
        claim.kind = if s.contains("decid") {
            Kind::Decision
        } else {
            Kind::Project
        };
        claim.subject = format!("project.{p}");
        claim.property = "database".into();
        claim.project_hint = Some(p.clone());
        claim.value = ["sqlite", "postgresql", "mysql"]
            .into_iter()
            .find(|v| s.contains(v))
            .unwrap()
            .into();
        claim.content = format!("O projeto {p} utiliza {}.", claim.value);
        claim.rule = "explicit_project_database".into();
    } else if contains_any(&s, &["meu objetivo e ", "my goal is "]) {
        claim.kind = Kind::Goal;
        claim.property = format!("goal.{}", &digest(&s)[..12]);
        claim.value = s.clone();
        claim.rule = "explicit_goal".into();
    } else if contains_any(&s, &["preciso ", "tenho que ", "i need to "])
        && contains_any(&s, &["amanha", "hoje", "tomorrow", "today"])
    {
        claim.kind = Kind::Task;
        claim.property = format!("task.{}", &digest(&s)[..12]);
        claim.value = s.clone();
        claim.temporal = "task".into();
        claim.ttl_seconds = Some(2 * 86400);
        claim.rule = "explicit_temporary_task".into();
    } else if contains_any(&s, &["nao posso usar ", "we must not use "]) {
        claim.kind = Kind::Constraint;
        claim.property = format!("constraint.{}", &digest(&s)[..12]);
        claim.value = s.clone();
        claim.rule = "explicit_constraint".into();
    } else if contains_any(&s, &["resolvi o problema ", "i solved the issue "]) {
        claim.kind = Kind::Solution;
        claim.property = format!("solution.{}", &digest(&s)[..12]);
        claim.value = s.clone();
        claim.rule = "explicit_solution".into();
    } else {
        return None;
    }
    Some(claim)
}
pub fn preference_content(value: &str, facets: &[String]) -> String {
    let details = facets
        .iter()
        .map(|v| match v.as_str() {
            "rpg" => "RPG",
            "classes" => "classes",
            "progression" => "evolução de personagem",
            _ => "",
        })
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "{} jogos táticos por turnos{}.",
        if value == "negative" {
            "Não gosta de"
        } else {
            "Gosta de"
        },
        if details.is_empty() {
            String::new()
        } else {
            format!(" ({details})")
        }
    )
}
