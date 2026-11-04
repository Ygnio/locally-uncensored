//! Index descriptions are derived only from the current canonical record/version.
//! Bilingual labels translate documented schema values, never unknown source facts.
use crate::{
    intelligent::{quality, Quality},
    Memory, Result, Status, Store,
};
pub const REPRESENTATION_VERSION: u32 = 5;
pub fn render(memory: &Memory, q: Option<&Quality>, style: &str) -> Result<String> {
    let original = memory.input.content.clone();
    let Some(q) = q.filter(|q| q.evidence_rule != "manual_revision") else {
        return Ok(original);
    };
    let project_modality = match q.temporal.as_str() {
        "planned" => "plans to use",
        "current" => "currently uses",
        "in_progress" => "is implementing",
        "completed" => "completed implementation with",
        "cancelled" => "cancelled implementation with",
        "unknown" => "explicitly has not decided",
        _ => "decided to use",
    };
    let natural = match (q.property.as_str(), q.value.as_str()) {
        ("coffee_sugar", "without_sugar") => "User prefers unsweetened coffee without sugar. Prefere café sem açúcar.".into(),
        ("coffee_sugar", "with_sugar") => "User prefers coffee with sugar. Prefere café com açúcar.".into(),
        ("coffee_sugar", "dislikes_coffee") => "User no longer likes coffee. Não gosta mais de café.".into(),
        ("response_language", "pt") => "User prefers answers in Portuguese. Prefere respostas em português.".into(),
        ("games_squad_tactics", "positive") => "User prefers turn-based squad tactics games. Prefere jogos de tática por turnos.".into(),
        ("games_branching_story", "positive") => "User prefers campaigns where choices change the story. Prefere campanhas com decisões que alteram a história.".into(),
        ("games_tactics", value @ ("positive" | "negative")) => {
            let mut text=format!("User {} tactical turn-based games.",if value=="positive" {"likes"}else{"does not like"});
            if q.facets.iter().any(|f|f=="rpg") {text.push_str(" Role-playing RPG.")}
            if q.facets.iter().any(|f|f=="classes") {text.push_str(" Character classes.")}
            if q.facets.iter().any(|f|f=="progression") {text.push_str(" Character progression.")}
            format!("{text} {original}")
        },
        ("ram_capacity" | "free_ram", value) if value.ends_with("_gb") => {
            let device=match q.subject.as_str(){"device.primary_computer"=>"primary computer / computador principal","device.work_laptop"=>"work laptop / notebook do trabalho","device.other_computer"=>"other computer / outro computador",_=>return Ok(original)};
            let number=value.trim_end_matches("_gb");
            let property=if q.property=="free_ram" {"free RAM available now / memória RAM livre agora"}else{"installed RAM capacity / capacidade de RAM instalada"};
            format!("User's {device}: {number} GB {property}.")
        },
        ("language", value @ ("python"|"rust"|"typescript"|"javascript")) if q.subject.starts_with("project.") => format!("Project {} {} {value} as its programming language. {original}",q.subject.trim_start_matches("project."),project_modality),
        ("interface_theme", "dark")=>"User prefers the dark interface theme. Prefere interface em modo escuro.".into(),
        ("interface_theme", "light")=>"User prefers the light interface theme. Prefere interface em modo claro.".into(),
        ("communication_style", "concise")=>"User prefers concise short answers. Prefere respostas curtas e concisas.".into(),
        ("favorite_beverage", "tea")=>"User's favorite beverage is tea. Bebida favorita: chá.".into(),
        ("communication_focus", "objective")=>"User prefers objective answers. Prefere respostas objetivas; length is unspecified.".into(),
        ("interface_clutter", "negative")=>"User dislikes cluttered interfaces. Detesta interface poluída.".into(),
        ("games_character_creation", "positive")=>"User likes creating characters in games. Gosta de criar personagens.".into(),
        ("games_class_selection", "positive")=>"User likes choosing character classes. Gosta de escolher classes.".into(),
        ("hobby", "acoustic_guitar")=>"User prefers playing acoustic guitar. Prefere tocar violão acústico.".into(),
        ("music_instrument", "acoustic_guitar")=>"User prefers acoustic guitar. Prefere violão acústico.".into(),
        ("book_genre", "adventure")=>"User prefers adventure novels. Prefere romances de aventura.".into(),
        ("outing", "short_trails")=>"User prefers short trails. Prefere trilhas curtas.".into(),
        ("photo_paper", "glossy")=>"User prefers glossy photo prints. Prefere impressões fotográficas brilhantes.".into(),
        ("shelf_material", "wood")=>"User prefers wooden bookshelves. Prefere estantes de madeira.".into(),
        ("communication_style", "detailed")=>"User prefers detailed answers. Prefere respostas detalhadas.".into(),
        ("games_turn_strategy", "positive")=>"User likes turn-based strategy games. Prefere jogos de estratégia por turnos.".into(),
        ("database", value @ ("sqlite" | "postgresql" | "mysql")) if q.subject.starts_with("project.") => format!("Project {} {} {value} as its database. {original}",q.subject.trim_start_matches("project."),project_modality),
        _ => format!("{}: {original}",match memory.input.kind {crate::Kind::Goal if q.temporal=="completed"=>"Completed goal / objetivo concluído",crate::Kind::Goal=>"User goal / objetivo",crate::Kind::Task=>"Dated user task / tarefa",crate::Kind::Solution=>"Resolved problem / solução",crate::Kind::Hypothesis=>"Uncertain intention, not installed hardware / hipótese",crate::Kind::Constraint=>"Constraint / restrição",_=>"Current user record"}),
    };
    match style {
        "original" | "canonical" => Ok(original),
        "structured" => Ok(format!(
            "{} | {} | {} | {} | {}",
            q.subject, q.property, q.value, q.temporal, original
        )),
        "natural" => Ok(natural),
        "gloss" => {
            let folded = crate::extraction::fold(&original);
            let mut terms = folded
                .split(|c: char| !c.is_alphanumeric())
                .filter_map(|word| match word {
                    "objetivo" => Some("goal"),
                    "terminar" | "concluir" => Some("finish"),
                    "paisagem" => Some("landscape"),
                    "aquarela" => Some("watercolor"),
                    "pintura" => Some("painting"),
                    "aprender" => Some("learn"),
                    "estudar" => Some("study"),
                    "italiano" => Some("Italian"),
                    "relatorio" => Some("report"),
                    "financeiro" => Some("financial"),
                    "amanha" => Some("tomorrow"),
                    "hoje" => Some("today"),
                    "problema" => Some("issue"),
                    "autenticacao" => Some("authentication"),
                    "certificado" => Some("certificate"),
                    "servidor" => Some("server"),
                    "trocando" => Some("replacing"),
                    "cartografia" => Some("cartography"),
                    "mapas" => Some("maps"),
                    "revisar" => Some("review"),
                    "orcamento" => Some("budget"),
                    "nao" => Some("not"),
                    _ => None,
                })
                .collect::<Vec<_>>();
            terms.sort();
            terms.dedup();
            Ok(if terms.is_empty() {
                natural
            } else {
                format!("{natural} Literal source gloss: {}.", terms.join(" "))
            })
        }
        "contextual" => Ok(format!(
            "Project {}. {:?}. {}",
            memory.input.project, memory.input.kind, natural
        )),
        _ => Err("Unknown embedding representation".into()),
    }
}
impl Store {
    pub fn memory_embedding_text(
        &self,
        id: &str,
        scope: &str,
        project: &str,
        style: &str,
    ) -> Result<serde_json::Value> {
        let memory = self.get(id, scope, project)?;
        if memory.state != Status::Active {
            return Err("Only current approved records can be indexed".into());
        }
        let meta = quality(&self.db, id)?;
        let mut text = render(&memory, meta.as_ref(), style)?;
        if style == "original" {
            if let Some(q) = meta
                .as_ref()
                .filter(|q| q.evidence_rule != "manual_revision")
            {
                let mut stmt=self.db.prepare("SELECT evidence FROM memory_provenance WHERE memory_id=?1 ORDER BY at DESC LIMIT 32").map_err(crate::db_error)?;
                let rows = stmt
                    .query_map([id], |r| r.get::<_, String>(0))
                    .map_err(crate::db_error)?;
                for row in rows {
                    let evidence = row.map_err(crate::db_error)?;
                    if crate::extraction::extract(&evidence).iter().any(|c| {
                        c.subject == q.subject && c.property == q.property && c.value == q.value
                    }) {
                        text = evidence;
                        break;
                    }
                }
            }
        }
        Ok(
            serde_json::json!({"id":id,"version":memory.version,"representation_version":REPRESENTATION_VERSION,"style":style,"text":text}),
        )
    }
}
