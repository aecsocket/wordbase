#![allow(dead_code)] // todo

use {
    crate::{Engine, IndexMap, IndexSet, lang},
    anyhow::{Context, Result, anyhow, bail},
    arc_swap::ArcSwap,
    client::{AnkiClient, VERSION},
    itertools::Itertools,
    maud::html,
    request::{Asset, DeckName, ModelFieldName, ModelName},
    serde::{Deserialize, Serialize},
    sqlx::{Pool, Sqlite},
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
    config: ArcSwap<AnkiConfig>,
    state: ArcSwap<Result<Arc<AnkiState>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnkiConfig {
    pub server_url: Arc<str>,
    pub api_key: Arc<str>,
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
    pub async fn new(db: &Pool<Sqlite>) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            // AnkiConnect HTTP server seems to eagerly close connections
            // so to prevent erroring when attempting to reuse a closed connection,
            // we won't pool connections
            .pool_max_idle_per_host(0)
            .build()
            .context("failed to create HTTP client")?;
        let config = anki_config(db)
            .await
            .context("failed to fetch Anki config")?;
        let state = fetch_anki_state(http_client.clone(), &config).await;

        Ok(Self {
            http_client,
            config: ArcSwap::from_pointee(config),
            state: ArcSwap::from_pointee(state),
        })
    }
}

impl Engine {
    #[must_use]
    pub fn anki_config(&self) -> Arc<AnkiConfig> {
        self.anki.config.load().clone()
    }

    pub fn anki_state(&self) -> Result<Arc<AnkiState>> {
        match &*self.anki.state.load().clone() {
            Ok(state) => Ok(state.clone()),
            Err(err) => Err(anyhow!("{err:?}")),
        }
    }

    pub async fn connect_anki(&self, config: Arc<AnkiConfig>) -> Result<()> {
        let url = &*config.server_url;
        let api_key = &*config.api_key;
        sqlx::query!(
            "UPDATE config SET ankiconnect_url = $1, ankiconnect_api_key = $2",
            url,
            api_key
        )
        .execute(&self.db)
        .await
        .context("failed to update Anki config")?;
        self.anki.config.store(config);

        let state = Arc::new(
            fetch_anki_state(
                self.anki.http_client.clone(),
                &self.anki.config.load().clone(),
            )
            .await,
        );
        self.anki.state.store(state);
        Ok(())
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
        let anki = self.anki_state().context("failed to connect to Anki")?;
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
            .map(|audio| audio.data.as_str());

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

async fn anki_config(db: &Pool<Sqlite>) -> Result<AnkiConfig> {
    let record = sqlx::query!("SELECT ankiconnect_url, ankiconnect_api_key FROM config")
        .fetch_one(db)
        .await?;
    Ok(AnkiConfig {
        server_url: Arc::from(record.ankiconnect_url),
        api_key: Arc::from(record.ankiconnect_api_key),
    })
}

async fn fetch_anki_state(
    http_client: reqwest::Client,
    config: &AnkiConfig,
) -> Result<Arc<AnkiState>> {
    let client = AnkiClient {
        http_client,
        url: config.server_url.clone(),
        api_key: config.api_key.clone(),
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

    Ok(Arc::new(AnkiState {
        client,
        decks,
        models,
    }))
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
