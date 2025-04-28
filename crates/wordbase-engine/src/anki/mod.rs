#![allow(dead_code)] // todo

use {
    crate::{Engine, IndexMap},
    anyhow::{Context, Result, anyhow, bail},
    arc_swap::ArcSwap,
    client::{AnkiClient, VERSION},
    request::{DeckName, ModelFieldName, ModelName},
    serde::{Deserialize, Serialize},
    sqlx::{Pool, Sqlite},
    std::{cmp, sync::Arc},
    wordbase::{NormString, ProfileId},
};

mod client;
mod note;
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

    pub async fn anki_connect(
        &self,
        server_url: impl Into<Arc<str>>,
        api_key: impl Into<Arc<str>>,
    ) -> Result<()> {
        let server_url = server_url.into();
        let api_key = api_key.into();
        let server_url_str = &*server_url;
        let api_key_str = &*api_key;
        sqlx::query!(
            "UPDATE config SET ankiconnect_url = $1, ankiconnect_api_key = $2",
            server_url_str,
            api_key_str
        )
        .execute(&self.db)
        .await
        .context("failed to update Anki config")?;
        self.anki.config.store(Arc::new(AnkiConfig {
            server_url,
            api_key,
        }));

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

    pub async fn set_anki_deck(&self, profile_id: ProfileId, deck: Option<&str>) -> Result<()> {
        sqlx::query!(
            "UPDATE profile SET anki_deck = $1 WHERE id = $2",
            deck,
            profile_id.0
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
            profile_id.0
        )
        .execute(&self.db)
        .await?;

        self.sync_profiles().await?;
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
