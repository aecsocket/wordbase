use {
    crate::{Engine, IndexSet, lang},
    anyhow::{Context, Result},
    itertools::Itertools,
    maud::html,
    serde::Serialize,
    std::{collections::HashMap, ops::Range},
    wordbase_api::{
        DictionaryId, FrequencyValue, NormString, ProfileId, Record, RecordEntry, Term, dict,
    },
};

impl Engine {
    pub fn build_term_note(
        &self,
        sentence: &str,
        entries: &[RecordEntry],
        term: &Term,
    ) -> Result<TermNote> {
        let entries = entries
            .iter()
            .filter(|record| record.term == *term)
            .collect::<Vec<_>>();

        let span_min = entries
            .iter()
            .map(|entry| entry.span_bytes.start)
            .min()
            .context("no records")?;
        let span_max = entries
            .iter()
            .map(|entry| entry.span_bytes.end)
            .max()
            .context("no records")?;
        let term_span = (usize::try_from(span_min).unwrap_or(usize::MAX))
            ..(usize::try_from(span_max).unwrap_or(usize::MAX));

        let dictionaries = self.dictionaries();
        let dict_name = |dict_id: DictionaryId| {
            dictionaries
                .get(&dict_id)
                .map_or("?", |dict| dict.meta.name.as_str())
        };
        let glossaries = glossaries(&entries);

        Ok(TermNote {
            fields: [
                ("Expression", term_part(term.headword())),
                ("ExpressionReading", term_part(term.reading())),
                ("ExpressionFurigana", term_ruby_plain(term)),
                (
                    "Sentence",
                    sentence_cloze(sentence, term_span).unwrap_or_default(),
                ),
                // TODO: generate sentence furigana, like AJT does
                // this is kinda complicated though
                // I can't use AJT's code for this since it uses an incredibly copyleft license
                (
                    "MainDefinition",
                    glossaries.first().cloned().unwrap_or_default(),
                ),
                ("Glossary", all_glossaries(&glossaries)),
                ("IsWordAndSentenceCard", String::new()),
                ("IsClickCard", String::new()),
                ("IsSentenceCard", "x".into()),
                ("PitchPosition", pitch_positions(&entries)),
                ("Frequency", frequency_list(&entries, dict_name)),
                ("FreqSort", frequency_harmonic_mean(&entries)),
            ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), NoteField::String(v)))
            .chain(
                term_audio(&entries)
                    .map(|audio| ("ExpressionAudio".to_string(), NoteField::Audio(audio))),
            )
            .collect::<HashMap<_, _>>(),
        })
    }

    pub async fn set_anki_deck(&self, profile_id: ProfileId, deck: Option<&str>) -> Result<()> {
        sqlx::query!(
            "UPDATE profile SET anki_deck = $1 WHERE id = $2",
            deck,
            profile_id.0,
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        Ok(())
    }

    pub async fn set_anki_note_type(
        &self,
        profile_id: ProfileId,
        note_type: Option<&str>,
    ) -> Result<()> {
        sqlx::query!(
            "UPDATE profile SET anki_note_type = $1 WHERE id = $2",
            note_type,
            profile_id.0,
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
        Ok(())
    }
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TermNote {
    pub fields: HashMap<String, NoteField>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum NoteField {
    String(String),
    Audio(Vec<u8>),
}

fn term_part(part: Option<&NormString>) -> String {
    part.map(ToString::to_string).unwrap_or_default()
}

fn term_ruby_plain(term: &Term) -> String {
    match term {
        Term::Full(headword, reading) => {
            // Lapis does something a bit screwy with furigana.
            // "押し込む" -> "押[お]し 込[こ]む"
            // Notice:
            // - after kanji segments, there is "[{reading}]", and no space afterwards
            // - after kana segments, there is a space
            lang::jpn::furigana_parts(headword, reading)
                .map(|(headword_part, reading_part)| {
                    if reading_part.is_empty() {
                        format!("{headword_part} ")
                    } else {
                        format!("{headword_part}[{reading_part}]")
                    }
                })
                .join("")
        }
        Term::Headword(headword) => headword.to_string(),
        Term::Reading(reading) => reading.to_string(),
    }
}

fn term_audio(entries: &[&RecordEntry]) -> Option<Vec<u8>> {
    entries
        .iter()
        .find_map(|entry| match &entry.record {
            Record::YomichanAudioForvo(audio) => Some(&audio.audio),
            Record::YomichanAudioJpod(audio) => Some(&audio.audio),
            Record::YomichanAudioNhk16(audio) => Some(&audio.audio),
            Record::YomichanAudioShinmeikai8(audio) => Some(&audio.audio),
            _ => None,
        })
        .map(|audio| audio.data.to_vec())
}

fn sentence_cloze(sentence: &str, term_span: Range<usize>) -> Option<String> {
    let cloze_prefix = sentence.get(..term_span.start)?;
    let cloze_body = sentence.get(term_span.clone())?;
    let cloze_suffix = sentence.get(term_span.end..)?;
    Some(format!("{cloze_prefix}<b>{cloze_body}</b>{cloze_suffix}"))
}

fn glossaries(entries: &[&RecordEntry]) -> Vec<String> {
    entries
        .iter()
        .filter_map(|record| match &record.record {
            Record::YomitanGlossary(glossary) => Some(
                html! {
                    ul {
                        @for content in &glossary.content {
                            li {
                                (dict::yomitan::render_html(content))
                            }
                        }
                    }
                }
                .0,
            ),
            _ => None,
        })
        .collect::<Vec<_>>()
}

fn all_glossaries(glossaries: &[String]) -> String {
    html! {
        ul {
            @for glossary in glossaries {
                li {
                    (glossary)
                }
            }
        }
    }
    .0
}

fn pitch_positions(entries: &[&RecordEntry]) -> String {
    entries
        .iter()
        .filter_map(|entry| match &entry.record {
            Record::YomitanPitch(dict::yomitan::Pitch { position, .. }) => Some(*position),
            _ => None,
        })
        // collect into a set first to deduplicate positions
        // IndexSet to retain ordering
        .collect::<IndexSet<_>>()
        .into_iter()
        .map(|position| format!("[{}]", position.0))
        .join("")
}

fn frequency_list<'a>(
    entries: &[&RecordEntry],
    dict_name: impl Fn(DictionaryId) -> &'a str,
) -> String {
    entries
        .iter()
        .filter_map(|entry| match &entry.record {
            Record::YomitanFrequency(dict::yomitan::Frequency { value, display }) => {
                match (display, value) {
                    (Some(display), _) => Some((entry, display.clone())),
                    (None, Some(FrequencyValue::Rank(rank))) => Some((entry, format!("{rank} ↓"))),
                    (None, Some(FrequencyValue::Occurrence(occurrence))) => {
                        Some((entry, format!("{occurrence} ↑")))
                    }
                    (None, None) => None,
                }
            }
            _ => None,
        })
        .map(|(record, frequency)| {
            html! {
                li { (dict_name(record.source)) ": " (frequency) }
            }
            .0
        })
        .join("")
}

fn frequency_harmonic_mean(entries: &[&RecordEntry]) -> String {
    harmonic_mean(
        entries
            .iter()
            .filter_map(|entry| match &entry.record {
                Record::YomitanFrequency(dict::yomitan::Frequency {
                    // TODO: how do we handle occurrences?
                    value: Some(FrequencyValue::Rank(rank)),
                    ..
                }) => Some(*rank),
                _ => None,
            })
            .map(|rank| rank as f64),
    )
    .map_or(String::new(), |mean| format!("{mean:.0}"))
}

fn harmonic_mean(v: impl IntoIterator<Item = f64>) -> Option<f64> {
    let mut count = 0usize;
    let mut sum_reciprocals = 0.0;
    for n in v {
        if n > 0.0 {
            count += 1;
            sum_reciprocals += 1.0 / n;
        }
    }
    let mean = count as f64 / sum_reciprocals;
    if mean.is_normal() { Some(mean) } else { None }
}

#[cfg(feature = "uniffi")]
const _: () = {
    use crate::{FfiResult, Wordbase};

    #[uniffi::export(async_runtime = "tokio")]
    impl Wordbase {
        pub fn build_term_note(
            &self,
            sentence: &str,
            entries: &[RecordEntry],
            term: &Term,
        ) -> FfiResult<TermNote> {
            Ok(self.0.build_term_note(sentence, entries, term)?)
        }

        pub async fn set_anki_deck(
            &self,
            profile_id: ProfileId,
            deck: Option<String>,
        ) -> FfiResult<()> {
            Ok(self.0.set_anki_deck(profile_id, deck.as_deref()).await?)
        }

        pub async fn set_anki_note_type(
            &self,
            profile_id: ProfileId,
            note_type: Option<String>,
        ) -> FfiResult<()> {
            Ok(self
                .0
                .set_anki_note_type(profile_id, note_type.as_deref())
                .await?)
        }
    }
};
