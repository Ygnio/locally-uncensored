use lucille_memory::dialogue;

#[test]
fn reviewed_story_theme_is_a_fact_about_the_character() {
    let source = "No meu conto, defini que a personagem Íris prefere o tema sépia.";
    let claims = dialogue::extract(source, "lume");
    assert_eq!(claims.len(), 1);
    let c = &claims[0];
    assert_eq!((&*c.subject, &*c.property, &*c.value), ("character.iris", "fictional_theme", "sepia"));
    assert!(c.authority.as_ref().unwrap().supports(c));
    for other in [
        "A personagem Íris disse: prefiro o tema sépia.",
        "Talvez no meu conto a personagem Íris prefira o tema sépia.",
        "No meu conto, não defini que a personagem Íris prefere o tema sépia.",
    ] {
        assert!(!dialogue::extract(other, "lume").iter().any(|claim|
            ["fictional_theme", "interface_theme"].contains(&claim.property.as_str())));
    }
}

#[test]
fn correction_source_keeps_its_own_speaker_and_current_evidence() {
    let text = "Mudei de ideia sobre o banco do Alfenim: antes eu tinha escolhido SQLite, mas agora escolhi PostgreSQL. A migração ainda não começou.";
    let summary = lucille_memory::source_authority::speaker_summary(text).unwrap();
    assert_eq!(summary["personal"], 2);
    assert_eq!(summary["quoted"], 0);
    assert_eq!(summary["external"], 0);
    assert!(dialogue::extract(text, "alfenim").iter().any(|c| c.evidence == "agora escolhi PostgreSQL" && c.value == "postgresql"));
}
