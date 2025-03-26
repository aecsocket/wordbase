use std::cmp::Ordering;

use anyhow::{Context, Result};
use derive_more::{Display, Error, From};
use tokio::sync::Mutex;
use wordbase_ankiconnect::{VERSION, request};

use crate::Engine;

#[derive(Debug)]
pub(super) struct Anki {
    client: Mutex<Option<wordbase_ankiconnect::Client>>,
}

impl Anki {
    pub fn new() -> Self {
        Self {
            client: Mutex::new(None),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AnkiConfig {
    pub url: String,
    pub api_key: String,
}

#[derive(Debug, Display, Error, From)]
pub enum AnkiError {
    #[display("server version ({server_version}) is older than our version ({VERSION})")]
    ServerOutdated { server_version: u32 },
    #[display("server version ({server_version}) is newer than our version ({VERSION})")]
    ClientOutdated { server_version: u32 },
    #[from]
    Other(anyhow::Error),
}

impl Engine {
    pub async fn anki_config(&self) -> Result<AnkiConfig> {
        let record = sqlx::query!("SELECT ankiconnect_url, ankiconnect_api_key FROM config")
            .fetch_one(&self.db)
            .await?;
        Ok(AnkiConfig {
            url: record.ankiconnect_url,
            api_key: record.ankiconnect_api_key,
        })
    }

    pub async fn set_anki_config(&self, config: &AnkiConfig) -> Result<(), AnkiError> {
        let mut engine_client = self.anki.client.lock().await;
        *engine_client = None;

        sqlx::query!(
            "UPDATE config SET ankiconnect_url = $1, ankiconnect_api_key = $2",
            config.url,
            config.api_key
        )
        .execute(&self.db)
        .await
        .context("failed to update config")?;

        let client = wordbase_ankiconnect::Client {
            client: reqwest::Client::new(),
            url: config.url.clone(),
            api_key: if config.api_key.trim().is_empty() {
                None
            } else {
                Some(config.api_key.clone())
            },
        };
        let server_version = client
            .send(&request::Version)
            .await
            .context("failed to query server version")?;
        match VERSION.cmp(&server_version) {
            Ordering::Equal => {
                *engine_client = Some(client);
                drop(engine_client);
                Ok(())
            }
            Ordering::Greater => Err(AnkiError::ServerOutdated { server_version }),
            Ordering::Less => Err(AnkiError::ClientOutdated { server_version }),
        }
    }
}
