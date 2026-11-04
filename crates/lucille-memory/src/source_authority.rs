//! Source attribution before interpretation. Transport USER is not a speech author.
//! Pure UTF-8 source ranges; no model metadata, storage, network or permissions.
use crate::extraction::{digest, fold, Claim};
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Origin {
    PersonalDeclaration,
    ReportedSpeech,
    QuotedText,
    Uncertain,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SourceAuthority {
    pub message_author: String,
    pub statement_author: String,
    pub subject: String,
    pub origin: Origin,
    pub source_digest: String,
    pub evidence_digest: String,
    pub start: usize,
    pub end: usize,
    pub claim_digest: String,
}
fn binding(c: &Claim) -> String {
    digest(
        &serde_json::to_string(&(
            &c.kind,
            &c.subject,
            &c.property,
            &c.value,
            &c.temporal,
            c.ttl_seconds,
            &c.facets,
            &c.evidence,
        ))
        .expect("bounded claim"),
    )
}
impl SourceAuthority {
    pub fn supports(&self, c: &Claim) -> bool {
        self.message_author == "user"
            && self.statement_author == "user"
            && self.origin == Origin::PersonalDeclaration
            && self.subject == c.subject
            && self.source_digest.len() == 64
            && self.source_digest.bytes().all(|b| b.is_ascii_hexdigit())
            && self.end <= 16000
            && !c.evidence.is_empty()
            && self.start.checked_add(c.evidence.len()) == Some(self.end)
            && self.evidence_digest == digest(&c.evidence)
            && self.claim_digest == binding(c)
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Clause<'a> {
    pub text: &'a str,
    pub span: Range<usize>,
    pub message_author: &'static str,
    pub statement_author: &'static str,
    pub origin: Origin,
}
#[derive(Debug)]
struct Word {
    text: String,
    start: usize,
    end: usize,
}
fn words(s: &str) -> Vec<Word> {
    let mut result = vec![];
    let mut start = None;
    for (at, ch) in s.char_indices().chain(std::iter::once((s.len(), ' '))) {
        if ch.is_alphanumeric() {
            if start.is_none() {
                start = Some(at)
            }
        } else if let Some(begin) = start.take() {
            result.push(Word {
                text: fold(&s[begin..at]),
                start: begin,
                end: at,
            });
        }
    }
    result
}
fn member(w: &str, choices: &[&str]) -> bool {
    choices.contains(&w)
}
fn role_tag(s: &str) -> bool {
    [
        "user",
        "assistant",
        "tool",
        "system",
        "developer",
        "usuario",
        "assistente",
        "ferramenta",
    ]
    .contains(&fold(s).as_str())
}
fn foreign_noun(w: &str) -> bool {
    member(
        w,
        &[
            "irmao",
            "irma",
            "brother",
            "sister",
            "colega",
            "colleague",
            "vizinho",
            "vizinha",
            "neighbor",
            "neighbour",
            "amigo",
            "amiga",
            "friend",
            "personagem",
            "character",
            "narrador",
            "narradora",
            "narrator",
            "protagonista",
            "protagonist",
            "ferramenta",
            "tool",
            "log",
            "documento",
            "document",
            "manual",
            "roteiro",
            "script",
            "texto",
            "text",
            "trecho",
            "excerpt",
            "output",
            "saida",
            "resultado",
            "result",
            "mensagem",
            "message",
        ],
    )
}
fn speech(w: &str) -> bool {
    member(
        w,
        &[
            "diz",
            "disse",
            "dizem",
            "fala",
            "falou",
            "afirma",
            "afirmou",
            "informa",
            "informou",
            "relata",
            "relatou",
            "declara",
            "declarou",
            "conta",
            "contou",
            "escreve",
            "escreveu",
            "comenta",
            "comentou",
            "menciona",
            "mencionou",
            "retorna",
            "retornou",
            "contem",
            "copiado",
            "copiada",
            "extraido",
            "extraida",
            "inventei",
            "says",
            "said",
            "states",
            "stated",
            "tells",
            "told",
            "writes",
            "wrote",
            "reads",
            "read",
            "reports",
            "reported",
            "contains",
            "copied",
            "returned",
            "outputs",
            "invented",
        ],
    )
}
fn leading_foreign(ws: &[Word]) -> bool {
    let mut at = 0;
    while ws.get(at).is_some_and(|w| {
        member(
            &w.text,
            &[
                "a", "o", "as", "os", "the", "um", "uma", "no", "na", "nos", "nas", "in", "from",
                "do", "da", "of", "meu", "minha", "my",
            ],
        )
    }) {
        at += 1
    }
    ws.get(at).is_some_and(|w| {
        foreign_noun(&w.text)
            || member(
                &w.text,
                &[
                    "ele", "ela", "eles", "elas", "he", "she", "they", "someone", "alguem",
                ],
            )
    })
}
fn external(s: &str) -> bool {
    let ws = words(s);
    if leading_foreign(&ws) {
        return true;
    }
    if fictional_role(s) {
        return true;
    }
    let first_self = ws
        .iter()
        .position(|w| {
            member(
                &w.text,
                &[
                    "prefiro", "gosto", "adoro", "curto", "tenho", "meu", "minha", "meus",
                    "minhas", "my", "i",
                ],
            )
        })
        .unwrap_or(ws.len());
    let actor = ws.iter().position(|w| foreign_noun(&w.text));
    actor.is_some_and(|at| at < first_self && ws.iter().any(|w| speech(&w.text)))
        || ws
            .iter()
            .position(|w| speech(&w.text))
            .is_some_and(|at| at < first_self)
}
fn fictional_role(s: &str) -> bool {
    let s = fold(s);
    let first_relation = ["prefiro", "gosto", "i prefer", "i like", "i enjoy"]
        .iter()
        .filter_map(|w| s.find(w))
        .min()
        .unwrap_or(s.len());
    let prefix = &s[..first_relation];
    [
        "como personagem",
        "como narrador",
        "no papel de",
        "as a character",
        "as the narrator",
        "in the role of",
        "sou uma personagem",
        "sou um personagem",
        "sou uma narradora",
        "sou um narrador",
        "i am a fictional",
        "i'm a fictional",
        "roleplay",
        "role-play",
    ]
    .iter()
    .any(|v| prefix.contains(v))
}
fn declaration(s: &str) -> bool {
    let ws = words(s);
    ws.iter().any(|w|member(&w.text,&["eu","i","meu","minha","meus","minhas","my","mim","prefiro","gosto","adoro","curto","detesto","odeio","tenho","quero","preciso","resolvi","estou","planejo","pretendo","decidi","conclui","atualizei","troquei","volte"]))
        || ws.iter().skip_while(|w|member(&w.text,&["o","a","the"])).next().is_some_and(|w|member(&w.text,&["computador","pc","notebook","laptop","desktop","projeto","project"]))
        || ["na verdade", "actually,", "actually "].iter().any(|v|fold(s).starts_with(v))
        // Reuse the existing bounded declarative grammar rather than inventing
        // another list of every first-person verb/correction/constraint form.
        || !crate::extraction::extract(s).is_empty()
        || crate::dialogue::preference_authorship(s)
}
fn quote_end(text: &str, begin: usize, close: &str) -> Option<usize> {
    let mut from = begin;
    while let Some(offset) = text[from..].find(close) {
        let end = from + offset;
        let escaped = text[..end].chars().rev().take_while(|c| *c == '\\').count() % 2 == 1;
        let contraction = ["'", "’"].contains(&close)
            && text[..end]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric())
            && text[end + close.len()..]
                .chars()
                .next()
                .is_some_and(|c| c.is_alphanumeric());
        if !escaped && !contraction {
            return Some(end);
        }
        from = end + close.len();
    }
    None
}
fn trim_range(text: &str, range: Range<usize>) -> Range<usize> {
    let part = &text[range.clone()];
    let begin = part.len()
        - part
            .trim_start_matches(|c: char| c.is_whitespace() || c == ',')
            .len();
    let end = part
        .trim_end_matches(|c: char| c.is_whitespace() || c == ',')
        .len();
    range.start + begin..range.start + end.max(begin)
}
fn push<'a>(out: &mut Vec<Clause<'a>>, text: &'a str, range: Range<usize>, origin: Origin) {
    let span = trim_range(text, range);
    if span.is_empty() {
        return;
    }
    out.push(Clause {
        text: &text[span.clone()],
        span,
        message_author: "user",
        statement_author: if origin == Origin::PersonalDeclaration {
            "user"
        } else {
            "external_or_unproved"
        },
        origin,
    });
}
fn coordinated<'a>(
    out: &mut Vec<Clause<'a>>,
    text: &'a str,
    range: Range<usize>,
    reported: &mut bool,
) {
    let range = trim_range(text, range);
    if range.is_empty() {
        return;
    }
    let s = &text[range.clone()];
    let ws = words(s);
    let mut begin = range.start;
    for (at, w) in ws.iter().enumerate() {
        let adversative = member(&w.text, &["mas", "but", "porem", "whereas"]);
        let new_subject = member(&w.text, &["e", "and"]) && leading_foreign(&ws[at + 1..]);
        if adversative || new_subject {
            attributed(out, text, begin..range.start + w.start, reported);
            begin = range.start + w.end;
        }
    }
    attributed(out, text, begin..range.end, reported);
}
fn attributed<'a>(
    out: &mut Vec<Clause<'a>>,
    text: &'a str,
    range: Range<usize>,
    reported: &mut bool,
) {
    let range = trim_range(text, range);
    if range.is_empty() {
        return;
    }
    let s = &text[range.clone()];
    if fictional_role(s) {
        *reported = true
    }
    // Colon speech frames bind following unquoted first-person text, including
    // a following physical line. Personal memory directives are not speech frames.
    if let Some(colon) = s.find(':') {
        if external(&s[..colon]) || role_tag(&s[..colon]) {
            push(
                out,
                text,
                range.start..range.start + colon,
                Origin::ReportedSpeech,
            );
            *reported = true;
            push(
                out,
                text,
                range.start + colon + 1..range.end,
                Origin::ReportedSpeech,
            );
            return;
        }
    }
    let origin = if *reported || external(s) {
        Origin::ReportedSpeech
    } else if declaration(s) {
        Origin::PersonalDeclaration
    } else {
        Origin::Uncertain
    };
    if origin == Origin::ReportedSpeech && s.ends_with(':') {
        *reported = true
    }
    push(out, text, range, origin)
}
/// Resolve structural quotation/report/coordination before any fact extractor.
/// Quoted input never changes the speaker of an adjacent personal assertion.
pub fn clauses(text: &str) -> Vec<Clause<'_>> {
    if text.len() > 16000 {
        return vec![];
    }
    let mut out = vec![];
    let mut begin = 0;
    let mut at = 0;
    let mut reported = false;
    let mut blockquote = false;
    while at < text.len() {
        if blockquote && (at == 0 || text[..at].ends_with('\n')) {
            let end = text[at..].find('\n').map(|v| at + v).unwrap_or(text.len());
            if text[at..end].trim().is_empty() {
                blockquote = false
            } else {
                push(&mut out, text, at..end, Origin::QuotedText);
                at = end;
                begin = end;
                continue;
            }
        }
        let ch = text[at..].chars().next().unwrap();
        let next = at + ch.len_utf8();
        let fence = text[at..].starts_with("```");
        let apostrophe = ch == '\''
            && text[..at]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_alphanumeric());
        let quote = if fence {
            Some("```")
        } else {
            match ch {
                '"' => Some("\""),
                '“' => Some("”"),
                '‘' => Some("’"),
                '«' => Some("»"),
                '「' => Some("」"),
                '`' => Some("`"),
                '\'' if !apostrophe => Some("'"),
                _ => None,
            }
        };
        if let Some(close) = quote {
            coordinated(&mut out, text, begin..at, &mut reported);
            let opening = if fence { 3 } else { ch.len_utf8() };
            let content = at + opening;
            let end = quote_end(text, content, close).unwrap_or(text.len());
            push(&mut out, text, content..end, Origin::QuotedText);
            at = if end < text.len() {
                end + close.len()
            } else {
                end
            };
            begin = at;
            // A closed quote supplies its own extent; it does not taint later
            // independent USER speech. Unclosed colon blocks remain reported.
            if end < text.len()
                && !out.iter().any(|part| {
                    part.origin != Origin::QuotedText
                        && (fictional_role(part.text) || role_tag(part.text))
                })
            {
                reported = false
            }
            continue;
        }
        if ch == '>' && (at == 0 || text[..at].ends_with('\n')) {
            coordinated(&mut out, text, begin..at, &mut reported);
            let end = text[at..].find('\n').map(|v| at + v).unwrap_or(text.len());
            push(&mut out, text, at..end, Origin::QuotedText);
            at = end;
            begin = end;
            blockquote = true;
            continue;
        }
        if ['.', '!', ';', '\n'].contains(&ch) {
            coordinated(&mut out, text, begin..at, &mut reported);
            begin = next;
        }
        at = next;
    }
    coordinated(&mut out, text, begin..text.len(), &mut reported);
    // Explicit backward attribution cannot be stripped by sentence nomination.
    // This targets the referenced assertion, not every personal clause in the message.
    for index in 0..out.len() {
        let s = fold(out[index].text);
        let backward = [
            "essa frase",
            "esta frase",
            "essa afirmacao",
            "esta afirmacao",
            "a frase anterior",
            "a declaracao anterior",
            "that statement",
            "that sentence",
            "the previous statement",
            "the previous sentence",
        ]
        .iter()
        .any(|v| s.contains(v));
        let other = [
            "outra pessoa",
            "not mine",
            "nao e minha",
            "nao e meu",
            "pertence",
            "belongs",
            "said by",
            "spoken by",
        ]
        .iter()
        .any(|v| s.contains(v))
            || leading_foreign(&words(s.split(" e ").nth(1).unwrap_or("")));
        if backward && other && !s.contains("tambem") && !s.contains("also") {
            if let Some(prior) = out[..index]
                .iter_mut()
                .rev()
                .find(|part| part.origin != Origin::Uncertain)
            {
                if prior.origin == Origin::PersonalDeclaration {
                    prior.origin = Origin::ReportedSpeech;
                    prior.statement_author = "external_or_unproved"
                }
            }
        }
    }
    out
}
/// Discard any caller-supplied metadata; only original native USER source grants proof.
pub(crate) fn attest(source: &str, mut c: Claim) -> Option<Claim> {
    c.authority = None;
    if c.evidence.is_empty() || c.evidence.len() > 300 {
        return None;
    }
    let candidates = clauses(source);
    let owned = candidates.iter().find_map(|part| {
        if part.origin != Origin::PersonalDeclaration {
            return None;
        }
        part.text
            .find(&c.evidence)
            .map(|offset| part.span.start + offset)
    })?;
    c.authority = Some(SourceAuthority {
        message_author: "user".into(),
        statement_author: "user".into(),
        subject: c.subject.clone(),
        origin: Origin::PersonalDeclaration,
        source_digest: digest(source),
        evidence_digest: digest(&c.evidence),
        start: owned,
        end: owned + c.evidence.len(),
        claim_digest: binding(&c),
    });
    Some(c)
}
pub(crate) fn valid(c: &Claim) -> bool {
    c.authority.as_ref().is_some_and(|p| p.supports(c))
}
pub(crate) fn independently_interpreted(c: &Claim, project: &str) -> bool {
    crate::extraction::extract(&c.evidence)
        .into_iter()
        .chain(crate::dialogue::extract(&c.evidence, project))
        .any(|mut interpreted| {
            let mut meaning = c.clone();
            meaning.authority = None;
            interpreted.authority = None;
            interpreted == meaning
        })
}
pub(crate) fn source_job_id(
    conversation: &str,
    message: &str,
    scope: &str,
    project: &str,
    source_digest: &str,
) -> String {
    digest(
        &serde_json::to_string(&(conversation, message, scope, project, source_digest))
            .expect("bounded source binding"),
    )
}
pub(crate) fn bound_to_job(
    c: &Claim,
    id: &str,
    conversation: &str,
    message: &str,
    scope: &str,
    project: &str,
) -> bool {
    valid(c)
        && c.authority.as_ref().is_some_and(|p| {
            source_job_id(conversation, message, scope, project, &p.source_digest) == id
        })
}
/// Native, scoped resolution of a previously proven underspecified hardware target.
pub(crate) fn rebind(original: &Claim, resolved: &mut Claim) {
    if valid(original) {
        let mut proof = original.authority.clone().unwrap();
        proof.subject = resolved.subject.clone();
        proof.claim_digest = binding(resolved);
        resolved.authority = Some(proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn speaker_is_per_clause_and_original_utf8_ranges_are_retained() {
        for text in [
            "Eu prefiro café, mas a personagem do jogo prefere chá.",
            "Eu prefiro café sem açúcar. Minha irmã disse: \"Eu prefiro chá\".",
            "I prefer coffee, but my brother prefers tea.",
        ] {
            let parts = clauses(text);
            assert!(parts.iter().any(|c| c.origin == Origin::PersonalDeclaration
                && (c.text.contains("café") || c.text.contains("coffee"))));
            for part in parts {
                assert_eq!(part.text, &text[part.span.clone()]);
                if part.text.contains("chá") || part.text.contains("tea") {
                    assert_ne!(part.origin, Origin::PersonalDeclaration)
                }
            }
        }
        for text in [
            "A mensagem de ferramenta informa: eu prefiro chá como bebida favorita.",
            "No diálogo que inventei, a narradora diz eu gosto de café sem açúcar.",
            "Trecho do documento:\nEu prefiro café sem açúcar.",
            "A narradora apresenta seu depoimento:\nEu prefiro café sem açúcar.",
        ] {
            assert!(
                !clauses(text)
                    .iter()
                    .any(|c| c.origin == Origin::PersonalDeclaration
                        && (c.text.contains("prefiro") || c.text.contains("gosto"))),
                "{text}"
            )
        }
    }
}
