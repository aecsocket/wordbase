use std::{fmt::Write, iter, time::Instant};

use anyhow::Result;
use itertools::Itertools;
use tracing::info;
use wordbase::{
    DictionaryId, Engine, FrequencyValue, Profile, RecordEntry, RecordKind,
    dict::{self, jpn::PitchPosition},
    dictionary::Dictionaries,
    render,
};

pub async fn lookup(engine: &Engine, profile: &Profile, text: &str) -> Result<Vec<RecordEntry>> {
    let start = Instant::now();
    let entries = engine.lookup(profile.id, text, 0, RecordKind::ALL).await?;
    let end = Instant::now();

    let dictionaries = engine.dictionaries();
    let mut w = String::new();
    for term in render::group_terms(&entries) {
        render(&mut w, &dictionaries, &term)?;
    }
    info!("\n{w}");

    info!("Fetched records in {:?}", end.duration_since(start));
    Ok(entries)
}

fn render(mut w: impl Write, dictionaries: &Dictionaries, term: &render::RecordTerm) -> Result<()> {
    if term.term.reading().is_some() {
        let reading = term
            .info
            .furigana_parts
            .iter()
            .map(|(headword, reading)| {
                if reading.is_empty() {
                    headword.clone()
                } else {
                    format!("{headword}[{reading}]")
                }
            })
            .join(" ");
        writeln!(w, "{} - {reading}", term.term)?;
    } else {
        writeln!(w, "{}", term.term)?;
    }

    let tags = iter::empty()
        .chain(tags_pitch(term))
        .chain(tags_audio(dictionaries, term))
        .chain(tags_frequency(dictionaries, term))
        .map(|tag| format!("[{tag}]"))
        .join(" ");
    if !tags.is_empty() {
        writeln!(w, "  {tags}\n")?;
    }

    for (&source, glossary_group) in &term.info.glossary_groups {
        writeln!(w, "  {}:", dict_name(dictionaries, source))?;

        for glossary in glossary_group {
            for content in &glossary.content {
                writeln!(w, "    {}", content.chars().take(40).collect::<String>())?;
            }
        }
    }

    writeln!(w, "\n")?;
    Ok(())
}

fn tags_frequency<'a>(
    dictionaries: &'a Dictionaries,
    term: &'a render::RecordTerm,
) -> impl Iterator<Item = String> + 'a {
    term.info
        .frequencies
        .iter()
        .map(move |(&source, frequencies)| {
            format!(
                "{} {}",
                dict_name(dictionaries, source),
                frequencies
                    .iter()
                    .map(|freq| render_frequency(freq))
                    .join("・")
            )
        })
}

fn render_frequency(frequency: &dict::yomitan::Frequency) -> String {
    match (&frequency.display, &frequency.value) {
        (Some(display), _) => display.clone(),
        (None, Some(FrequencyValue::Rank(rank))) => format!("{rank} ↓"),
        (None, Some(FrequencyValue::Occurrence(occurrence))) => format!("{occurrence} ↑"),
        (None, None) => "?".into(),
    }
}

fn tags_pitch(term: &render::RecordTerm) -> impl Iterator<Item = String> {
    let morae = term
        .term
        .reading()
        .map(|s| dict::jpn::morae(s).collect::<Vec<_>>());

    term.info
        .pitches
        .iter()
        .map(|(position, pitch)| render_pitch(morae.as_deref(), *position, pitch))
        .collect::<Vec<_>>()
        .into_iter()
}

fn render_pitch(morae: Option<&[&str]>, position: PitchPosition, pitch: &render::Pitch) -> String {
    let reading = (|| {
        let mut morae = morae?.iter().zip(pitch.high.iter()).peekable();
        let mut last_high = *morae.peek()?.1;
        let mut reading = String::new();
        for (mora, &high) in morae {
            match (last_high, high) {
                (false, false) | (true, true) => {}
                (false, true) => {
                    _ = write!(reading, "／");
                }
                (true, false) => {
                    _ = write!(reading, "＼");
                }
            }
            last_high = high;
            _ = write!(reading, "{mora}");
        }
        Some(reading)
    })();

    let reading = reading.unwrap_or_else(|| format!("{}", position.0));
    iter::once(reading)
        .chain((0..pitch.audio.len()).map(|_| "🔊".to_string()))
        .join(" ")
}

fn tags_audio(
    dictionaries: &Dictionaries,
    term: &render::RecordTerm,
) -> impl Iterator<Item = String> {
    term.info
        .audio_no_pitch
        .iter()
        .map(|(&source, _)| format!("🔊 {}", dict_name(dictionaries, source)))
}

fn dict_name(dictionaries: &Dictionaries, id: DictionaryId) -> &str {
    dictionaries
        .get(&id)
        .map_or("?", |dict| dict.meta.name.as_str())
}
