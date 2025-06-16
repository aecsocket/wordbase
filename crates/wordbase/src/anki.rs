use {
    crate::{Engine, IndexSet, lang},
    anyhow::{Context, Result},
    itertools::Itertools,
    maud::html,
    serde::Serialize,
    std::{collections::HashMap, fmt::Write as _, ops::Range},
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
            .map(|record| record.span_bytes.start)
            .min()
            .context("no records")?;
        let span_max = entries
            .iter()
            .map(|record| record.span_bytes.end)
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
        let fields = [
            ("Expression", term_part(term.headword())),
            ("ExpressionReading", term_part(term.reading())),
            ("ExpressionFurigana", term_ruby_plain(term)),
            (
                "Sentence",
                sentence_cloze(sentence, term_span).unwrap_or_default(),
            ),
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
        ];

        Ok(TermNote {
            fields: fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
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
    pub fields: HashMap<String, String>,
}

fn term_part(part: Option<&NormString>) -> String {
    part.map(ToString::to_string).unwrap_or_default()
}

fn term_ruby_plain(term: &Term) -> String {
    match term {
        Term::Full(headword, reading) => {
            let mut result = String::new();
            for (headword_part, reading_part) in lang::jpn::furigana_parts(headword, reading) {
                _ = write!(&mut result, "{headword_part}");
                if !reading_part.is_empty() {
                    _ = write!(&mut result, "[{reading_part}]");
                }
                // Lapis uses a space to separate headword/reading part pairs
                // todo do this properly use this as ref: 落とし穴
                _ = write!(&mut result, " ");
            }
            result
        }
        Term::Headword(headword) => headword.to_string(),
        Term::Reading(reading) => reading.to_string(),
    }
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
        .filter_map(|record| match &record.record {
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
        .filter_map(|record| match &record.record {
            Record::YomitanFrequency(dict::yomitan::Frequency { value, display }) => {
                match (value, display) {
                    (_, Some(display)) => Some((record, display.clone())),
                    (Some(FrequencyValue::Rank(rank)), None) => Some((record, format!("{rank}"))),
                    _ => None,
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
            .filter_map(|record| match &record.record {
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
