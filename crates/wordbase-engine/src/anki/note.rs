use {
    super::request::{self, Asset},
    crate::{Engine, IndexSet, lang},
    anyhow::{Context as _, Result},
    data_encoding::BASE64,
    itertools::Itertools as _,
    maud::html,
    std::fmt::Write as _,
    wordbase::{
        DictionaryId, FrequencyValue, NormString, ProfileId, Record, RecordKind, Term, dict,
    },
};

impl Engine {
    pub async fn add_anki_note(
        &self,
        profile_id: ProfileId,
        sentence: &str,
        cursor: usize,
        term: &Term,
        sentence_audio: Option<&str>,
        sentence_image: Option<&str>,
    ) -> Result<()> {
        let anki = self.anki_state().context("failed to connect to Anki")?;
        let profile = self
            .profiles()
            .get(&profile_id)
            .cloned()
            .context("profile not found")?;
        let deck_name = profile.anki_deck.as_ref().context("no Anki deck name")?;
        let note_type_name = profile
            .anki_note_type
            .as_ref()
            .context("no Anki note type name")?;

        let records = self
            .lookup(profile_id, sentence, cursor, ANKI_RECORD_KINDS)
            .await?
            .into_iter()
            .filter(|record| record.term == *term)
            .collect::<Vec<_>>();
        let byte_scan_len = records
            .iter()
            .map(|record| record.bytes_scanned)
            .max()
            .context("no records")?;

        let dictionaries = self.dictionaries();
        let dict_name = |dict_id: DictionaryId| {
            dictionaries
                .get(&dict_id)
                .map_or("?", |dict| dict.meta.name.as_str())
        };

        // based on Lapis
        let term_ruby_plain = term_ruby_plain(term);
        let scan_end = cursor + byte_scan_len;
        let sentence_cloze = if let (Some(cloze_prefix), Some(cloze_body), Some(cloze_suffix)) = (
            sentence.get(..cursor),
            sentence.get(cursor..scan_end),
            sentence.get(scan_end..),
        ) {
            &format!("{cloze_prefix}<b>{cloze_body}</b>{cloze_suffix}")
        } else {
            ""
        };

        let pitch_positions = records
        .iter()
        .filter_map(|record| match &record.record {
            Record::YomitanPitch(dict::yomitan::Pitch { position, .. }) => Some(*position),
            _ => None,
        })
        // collect into a set first to deduplicate positions
        // IndexSet to retain ordering
        .collect::<IndexSet<_>>();
        let pitch_positions = pitch_positions
            .into_iter()
            .map(|position| format!("[{position}]"))
            .join("");

        let frequencies = records
            .iter()
            .filter_map(|record| match &record.record {
                Record::YomitanFrequency(dict::yomitan::Frequency { value, display }) => {
                    match (value, display) {
                        (_, Some(display)) => Some((record, display.clone())),
                        (Some(FrequencyValue::Rank(rank)), None) => {
                            Some((record, format!("{rank}")))
                        }
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
            .join("");

        let frequency_harmonic_mean = harmonic_mean(
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
        );
        #[expect(clippy::option_if_let_else, reason = "borrow checker")]
        let frequency_harmonic_mean = if let Some(mean) = frequency_harmonic_mean {
            &format!("{mean:.0}")
        } else {
            ""
        };

        let mut audio = Vec::new();
        let term_audio = records
            .iter()
            .find_map(|record| match &record.record {
                Record::YomichanAudioForvo(dict::yomichan_audio::Forvo { audio, .. })
                | Record::YomichanAudioJpod(dict::yomichan_audio::Jpod { audio })
                | Record::YomichanAudioNhk16(dict::yomichan_audio::Nhk16 { audio, .. })
                | Record::YomichanAudioShinmeikai8(dict::yomichan_audio::Shinmeikai8 {
                    audio,
                    ..
                }) => Some(audio),
                _ => None,
            })
            .map(|audio| (audio.format, BASE64.encode(&audio.data)));

        if let Some((format, data)) = &term_audio {
            let filename = format!("wordbase.{format}");
            audio.push(Asset {
                filename,
                data: Some(data),
                path: None,
                url: None,
                skip_hash: None,
                fields: vec!["ExpressionAudio"],
            });
        }

        // if let Some((format, data)) = &sentence_audio {
        //     audio.push(Asset {
        //         filename: &format!("wordbase.{format}"),
        //         data: Some(data),
        //         path: None,
        //         url: None,
        //         skip_hash: None,
        //         fields: vec!["SentenceAudio"],
        //     });
        // }

        let glossaries = records
            .iter()
            .filter_map(|record| match &record.record {
                Record::YomitanGlossary(glossary) => Some(html! {
                    ul {
                        @for content in &glossary.content {
                            li {
                                (content)
                            }
                        }
                    }
                }),
                _ => None,
            })
            .collect::<Vec<_>>();

        let main_glossary = glossaries.first().cloned().map(|s| s.0);
        let glossaries = html! {
            ul {
                @for glossary in &glossaries {
                    li {
                        (glossary)
                    }
                }
            }
        };

        let mut picture = Vec::new();
        if let Some(data) = sentence_image {
            picture.push(Asset {
                filename: "test.png".into(),
                data: Some(data),
                path: None,
                url: None,
                skip_hash: None,
                fields: vec!["Picture"],
            });
        }

        let note = request::Note {
            deck_name,
            model_name: note_type_name,
            fields: [
                ("Expression", as_str(term.headword())),
                ("ExpressionReading", as_str(term.reading())),
                ("ExpressionFurigana", &term_ruby_plain),
                ("Sentence", sentence_cloze),
                (
                    "MainDefinition",
                    main_glossary.as_deref().unwrap_or_default(),
                ),
                ("Glossary", &glossaries.0),
                ("IsWordAndSentenceCard", ""),
                ("IsClickCard", ""),
                ("IsSentenceCard", "x"),
                ("PitchPosition", &pitch_positions),
                ("Frequency", &frequencies),
                ("FreqSort", frequency_harmonic_mean),
            ]
            .into_iter()
            .collect(),
            options: request::NoteOptions {
                allow_duplicate: false,
                duplicate_scope: None,
                duplicate_scope_options: None,
            },
            tags: vec!["wordbase"],
            audio,
            video: Vec::new(),
            picture,
        };
        anki.client.send(&request::AddNote { note }).await?;
        Ok(())
    }
}

const ANKI_RECORD_KINDS: &[RecordKind] = RecordKind::ALL;

fn as_str(str: Option<&NormString>) -> &str {
    str.map(|s| &***s).unwrap_or_default()
}

fn term_ruby_plain(term: &Term) -> String {
    match term {
        Term::Headword { headword } => headword.to_string(),
        Term::Reading { reading } => reading.to_string(),
        Term::Full { headword, reading } => {
            let mut result = String::new();
            for (headword_part, reading_part) in lang::jpn::furigana_parts(headword, reading) {
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
    }
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
