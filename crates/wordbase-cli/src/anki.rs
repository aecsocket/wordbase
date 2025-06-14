use {
    anyhow::{Context, Result},
    ascii_table::AsciiTable,
    tracing::info,
    wordbase::{Engine, Profile, Term, anki::TermNote},
};

pub async fn note(
    engine: &Engine,
    profile: &Profile,
    headword: &str,
    sentence: Option<&str>,
    reading: Option<&str>,
) -> Result<TermNote> {
    let term = Term::from_parts(Some(headword), reading).context("invalid term")?;
    let sentence = sentence.unwrap_or(headword);
    let term_note = engine
        .build_term_note(profile.id, sentence, 0, &term)
        .await?;

    let mut table = AsciiTable::default();
    table.column(0).set_header("Field");
    table.column(1).set_header("Value");

    let data = term_note
        .fields
        .iter()
        .map(|(key, value)| vec![key, value])
        .collect::<Vec<_>>();
    info!("\n{}", table.format(&data));
    Ok(term_note)
}
