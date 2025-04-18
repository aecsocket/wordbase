#![allow(dead_code)] // todo

use {
    crate::{Engine, IndexMap, IndexSet, lang},
    anyhow::{Context, Result, bail},
    arc_swap::ArcSwapOption,
    client::{AnkiClient, VERSION},
    data_encoding::BASE64,
    itertools::Itertools,
    maud::html,
    request::{Asset, DeckName, ModelFieldName, ModelName},
    std::{cmp, fmt::Write as _, sync::Arc},
    wordbase::{
        DictionaryId, FrequencyValue, NormString, ProfileId, Record, RecordKind, Term, dict,
    },
};

mod client;
mod request;

#[derive(Debug)]
pub struct Anki {
    http_client: reqwest::Client,
    state: ArcSwapOption<AnkiState>,
}

#[derive(Debug)]
pub struct AnkiState {
    client: AnkiClient,
    pub decks: Vec<DeckName>,
    pub models: IndexMap<ModelName, Model>,
}

#[derive(Debug)]
pub struct Model {
    pub field_names: Vec<ModelFieldName>,
}

impl Anki {
    pub fn new() -> Result<Self> {
        Ok(Self {
            http_client: reqwest::Client::builder()
                .build()
                .context("failed to create HTTP client")?,
            state: ArcSwapOption::new(None),
        })
    }
}

impl Engine {
    // pub async fn connect_anki(
    //     &self,
    //     url: impl Into<String>,
    //     api_key: impl Into<String>,
    // ) -> Result<()> {
    //     let url = url.into();
    //     let api_key = api_key.into();
    //     sqlx::query!(
    //         "UPDATE config SET ankiconnect_url = $1, ankiconnect_api_key = $2",
    //         url,
    //         api_key
    //     )
    //     .execute(&self.db)
    //     .await
    //     .context("failed to update AnkiConnect config")?;

    //     self.sync_anki_state(url, api_key).await
    // }

    pub async fn anki_state(&self) -> Result<Arc<AnkiState>> {
        if let Some(state) = self.anki.state.load().clone() {
            return Ok(state);
        }

        let record = sqlx::query!("SELECT ankiconnect_url, ankiconnect_api_key FROM config")
            .fetch_one(&self.db)
            .await
            .context("failed to fetch config")?;

        let client = AnkiClient {
            http_client: self.anki.http_client.clone(),
            url: record.ankiconnect_url,
            api_key: record.ankiconnect_api_key,
        };
        let version = client
            .send(&request::Version)
            .await
            .context("failed to send version request")?;
        match version.cmp(&VERSION) {
            cmp::Ordering::Less => {
                bail!("server version ({version}) is older than ours ({VERSION})");
            }
            cmp::Ordering::Greater => {
                bail!("server version ({version}) is newer than ours ({VERSION})");
            }
            cmp::Ordering::Equal => {}
        }

        let decks = client
            .send(&request::DeckNames)
            .await
            .context("failed to fetch deck names")?;

        let mut models = IndexMap::default();
        let model_names = client
            .send(&request::ModelNames)
            .await
            .context("failed to fetch model names")?;
        for model_name in model_names {
            let field_names = client
                .send(&request::ModelFieldNames {
                    model_name: model_name.clone(),
                })
                .await
                .with_context(|| format!("failed to fetch model field names for {model_name:?}"))?;
            models.insert(model_name, Model { field_names });
        }

        let state = Arc::new(AnkiState {
            client,
            decks,
            models,
        });
        self.anki.state.store(Some(state.clone()));
        Ok(state)
    }

    pub async fn add_anki_note(
        &self,
        profile_id: ProfileId,
        sentence: &str,
        cursor: usize,
        term: &Term,
        sentence_audio: Option<&str>,
        sentence_image: Option<&str>,
    ) -> Result<()> {
        let anki = self
            .anki_state()
            .await
            .context("failed to connect to Anki")?;
        let profile = self
            .profiles()
            .get(&profile_id)
            .cloned()
            .context("profile not found")?;
        let deck_name = profile
            .config
            .anki_deck
            .as_ref()
            .context("no Anki deck name")?;
        let model_name = profile
            .config
            .anki_model
            .as_ref()
            .context("no Anki model name")?;

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
                | Record::YomichanAudioNhk16(dict::yomichan_audio::Nhk16 { audio })
                | Record::YomichanAudioShinmeikai8(dict::yomichan_audio::Shinmeikai8 {
                    audio,
                    ..
                }) => Some(audio),
                _ => None,
            })
            .map(|audio| BASE64.encode(&audio.data));

        if let Some(data) = &term_audio {
            audio.push(Asset {
                filename: "test",
                data: Some(data),
                path: None,
                url: None,
                skip_hash: None,
                fields: vec!["ExpressionAudio"],
            });
        }

        if let Some(data) = &sentence_audio {
            audio.push(Asset {
                filename: "test",
                data: Some(data),
                path: None,
                url: None,
                skip_hash: None,
                fields: vec!["SentenceAudio"],
            });
        }

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
                filename: "test.png",
                data: Some(data),
                path: None,
                url: None,
                skip_hash: None,
                fields: vec!["Picture"],
            });
        }

        let note = request::Note {
            deck_name,
            model_name,
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
                ("IsClickCard", "x"),
                ("IsSentenceCard", ""),
                ("PitchPosition", &pitch_positions),
                ("Frequency", &frequencies),
                ("FreqSort", frequency_harmonic_mean),
            ]
            .into_iter()
            .collect(),
            options: request::NoteOptions {
                allow_duplicate: true,
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
