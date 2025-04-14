#![allow(dead_code)] // todo

use {
    crate::{Engine, IndexMap},
    anyhow::{Context, Result, bail},
    arc_swap::ArcSwapOption,
    client::{AnkiClient, VERSION},
    request::{DeckName, ModelFieldName, ModelName},
    std::{cmp, sync::Arc},
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
        let http_client = reqwest::Client::builder()
            .build()
            .context("failed to create HTTP client")?;
        Ok(Self {
            http_client,
            state: ArcSwapOption::empty(),
        })
    }
}

impl Engine {
    #[must_use]
    pub fn anki_state(&self) -> Option<Arc<AnkiState>> {
        self.anki.state.load().clone()
    }

    pub async fn connect_anki(
        &self,
        url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Result<()> {
        let url = url.into();
        let api_key = api_key.into();
        sqlx::query!(
            "UPDATE config SET ankiconnect_url = $1, ankiconnect_api_key = $2",
            url,
            api_key
        )
        .execute(&self.db)
        .await
        .context("failed to update AnkiConnect config")?;

        self.sync_anki_state(url, api_key).await
    }

    async fn sync_anki_state(&self, url: String, api_key: String) -> Result<()> {
        let client = AnkiClient {
            http_client: self.anki.http_client.clone(),
            url,
            api_key,
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

        self.anki.state.store(Some(Arc::new(AnkiState {
            client,
            decks,
            models,
        })));
        Ok(())
    }

    // pub async fn create_anki_note(
    //     &self,
    //     profile_id: ProfileId,
    //     term: &Term,
    //     records: &[Record],
    // ) -> Result<()> {
    //     let anki = self
    //         .anki
    //         .state
    //         .load()
    //         .clone()
    //         .context("no Anki connection")?;
    //     let profiles = self.profiles();
    //     let current_profile = profiles.by_id.get(&profile_id).unwrap();

    //     // based on Lapis
    //     let fields = [
    //         ("Expr", "a"),
    //         // ("Expression", headword(term)),
    //         // ("ExpressionFurigana", term_ruby_plain(term)),
    //         // ("ExpressionReading", reading(term)),
    //         // ("MainDefinition", "to read"),
    //         // ("Sentence", "a <b>読む</b> b"),
    //         // ("SentenceAudio", ""),
    //         // ("Picture", ""),
    //         // ("Glossary", "glossary..."),
    //         // ("IsWordAndSentenceCard", ""),
    //         // ("IsClickCard", "x"),
    //         // ("IsSentenceCard", ""),
    //         // (
    //         //     "Frequency",
    //         //     r#"<ul style="text-align: left;"><li>JPDBv2㋕: 14122</li><li>BCCWJ:
    //         // 65310</li></ul>"#, ),
    //         // ("FreqSort", "22994"), // frequency harmonic mean
    //     ];

    //     let note = request::Note {
    //         deck_name: current_profile
    //             .config
    //             .anki_deck
    //             .as_ref()
    //             .context("no deck name")?,
    //         model_name: current_profile
    //             .config
    //             .anki_model
    //             .as_ref()
    //             .context("no model name")?,
    //         fields: fields.iter().map(|(k, v)| (*k, *v)).collect(),
    //         options: request::NoteOptions {
    //             allow_duplicate: true,
    //             duplicate_scope: None,
    //             duplicate_scope_options: None,
    //         },
    //         tags: vec!["wordbase".into()],
    //         audio: Vec::new(),
    //         video: Vec::new(),
    //         picture: Vec::new(),
    //     };

    //     println!("{}", serde_json::to_string(&note).unwrap());

    //     let fields = anki.client.send(&request::AddNote { note }).await?;
    //     Ok(())
    // }
}

// #[derive(Debug)]
// struct TemplateContext<'a> {
//     term: &'a Term,
// }

// fn headword(term: &Term) -> Box<str> {
//     term.headword()
//         .or_else(|| term.reading())
//         .map(|s| s.clone().into_inner())
//         .unwrap_or_default()
// }

// fn reading(term: &Term) -> Box<str> {
//     term.reading()
//         .map(|s| s.clone().into_inner())
//         .unwrap_or_default()
// }

// fn term_ruby_html(term: &Term) -> Box<str> {
//     todo!();
//     // html::render_term(term).0
// }

// fn term_ruby_plain(term: &Term) -> Box<str> {
//     match term {
//         Term::Headword { headword } => headword.to_string(),
//         Term::Reading { reading } => reading.to_string(),
//         Term::Full { headword, reading } => {
//             let mut result = String::new();
//             for (headword_part, reading_part) in lang::jpn::furigana_parts(headword, reading) {
//                 _ = write!(&mut result, "{headword_part}");
//                 if !reading_part.is_empty() {
//                     _ = write!(&mut result, "[{reading_part}]");
//                 }
//             }
//             result
//         }
//     }
// }
