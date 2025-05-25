use anyhow::{Context, Result};
use itertools::Itertools;
use maud::html;
use serde::Serialize;
use std::{collections::HashMap, fmt::Write as _};
use wordbase_api::{
    DictionaryId, FrequencyValue, NormString, ProfileId, Record, RecordKind, RecordLookup, Term,
    dict,
};

use crate::{Engine, IndexSet};

impl Engine {
    pub async fn build_term_note(
        &self,
        profile_id: ProfileId,
        sentence: &str,
        cursor: usize,
        term: &Term,
    ) -> Result<TermNote> {
        let records = self
            .lookup(profile_id, sentence, cursor, NOTE_RECORD_KINDS)
            .await
            .context("failed to look up records")?;
        let bytes_scanned = records
            .iter()
            .map(|record| record.bytes_scanned)
            .max()
            .context("no records")?;
        let bytes_scanned = usize::try_from(bytes_scanned).unwrap_or(usize::MAX);

        let dictionaries = self.dictionaries();
        let dict_name = |dict_id: DictionaryId| {
            dictionaries
                .get(&dict_id)
                .map_or("?", |dict| dict.meta.name.as_str())
        };
        let glossaries = glossaries(&records);
        let fields = [
            ("Expression", term_part(term.headword())),
            ("ExpressionReading", term_part(term.reading())),
            ("ExpressionFurigana", term_ruby_plain(term)),
            ("Sentence", sentence_cloze(sentence, cursor, bytes_scanned)),
            (
                "MainDefinition",
                glossaries.first().cloned().unwrap_or_default(),
            ),
            ("Glossary", all_glossaries(&glossaries)),
            ("IsWordAndSentenceCard", String::new()),
            ("IsClickCard", String::new()),
            ("IsSentenceCard", "x".into()),
            ("PitchPosition", pitch_positions(&records)),
            ("Frequency", frequency_list(&records, dict_name)),
            ("FreqSort", frequency_harmonic_mean(&records)),
        ];

        Ok(TermNote {
            fields: fields
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        })
    }
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TermNote {
    pub fields: HashMap<String, String>,
}

const NOTE_RECORD_KINDS: &[RecordKind] = RecordKind::ALL;

fn term_part(part: Option<&NormString>) -> String {
    part.map(ToString::to_string).unwrap_or_default()
}

fn term_ruby_plain(term: &Term) -> String {
    match term {
        Term::Full(headword, reading) => {
            let mut result = String::new();
            for (headword_part, reading_part) in dict::jpn::furigana_parts(headword, reading) {
                _ = write!(&mut result, "{headword_part}");
                if !reading_part.is_empty() {
                    _ = write!(&mut result, "[{reading_part}]");
                }
                // Lapis uses a space to separate headword/reading part pairs
                // todo tdo this properly use this as ref: 落とし穴
                _ = write!(&mut result, " ");
            }
            result
        }
        Term::Headword(headword) => headword.to_string(),
        Term::Reading(reading) => reading.to_string(),
    }
}

fn sentence_cloze(sentence: &str, cursor: usize, byte_scan_len: usize) -> String {
    let scan_end = cursor + byte_scan_len;
    (|| {
        let cloze_prefix = sentence.get(..cursor)?;
        let cloze_body = sentence.get(cursor..scan_end)?;
        let cloze_suffix = sentence.get(scan_end..)?;
        Some(format!("{cloze_prefix}<b>{cloze_body}</b>{cloze_suffix}"))
    })()
    .unwrap_or_default()
}

fn glossaries(records: &[RecordLookup]) -> Vec<String> {
    records
        .iter()
        .filter_map(|record| match &record.record {
            Record::YomitanGlossary(glossary) => Some(
                html! {
                    ul {
                        @for content in &glossary.content {
                            li {
                                (content)
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

fn pitch_positions(records: &[RecordLookup]) -> String {
    records
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
    records: &[RecordLookup],
    dict_name: impl Fn(DictionaryId) -> &'a str,
) -> String {
    records
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

fn frequency_harmonic_mean(records: &[RecordLookup]) -> String {
    harmonic_mean(
        records
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
        pub async fn build_term_note(
            &self,
            profile_id: ProfileId,
            sentence: &str,
            cursor: u64,
            term: &Term,
        ) -> FfiResult<TermNote> {
            let cursor = usize::try_from(cursor).context("cursor too large")?;
            Ok(self
                .0
                .build_term_note(profile_id, sentence, cursor, term)
                .await?)
        }
    }
};
