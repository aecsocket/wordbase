use {
    anyhow::{Context, Result},
    ascii_table::AsciiTable,
    tracing::info,
    wordbase::{
        Engine, Profile, Term,
        anki::{NoteField, TermNote},
    },
};

pub async fn note(
    engine: &Engine,
    profile: &Profile,
    headword: &str,
    sentence: Option<&str>,
    reading: Option<&str>,
) -> Result<TermNote> {
    let sentence = sentence.unwrap_or(headword);
    let term = Term::from_parts(Some(headword), reading).context("invalid term")?;

    let entries = engine
        .lookup(profile.id, sentence, 0)
        .await
        .context("failed to perform lookup")?;
    let term_note = engine.build_term_note(sentence, &entries, &term)?;

    let mut table = AsciiTable::default();
    table.column(0).set_header("Field");
    table.column(1).set_header("Value");

    let data = term_note
        .fields
        .iter()
        .map(|(key, value)| {
            vec![
                key,
                match value {
                    NoteField::String(s) => s,
                    NoteField::Audio(_) => "(binary data)",
                },
            ]
        })
        .collect::<Vec<_>>();
    info!("\n{}", table.format(&data));
    Ok(term_note)
}
