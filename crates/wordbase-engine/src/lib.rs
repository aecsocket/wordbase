#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

mod anki;
mod db;
mod deinflect;
mod dictionary;
pub mod import;
mod lookup;
mod profile;
mod texthook;

use {
    anyhow::{Context, Result},
    derive_more::{Deref, DerefMut},
    futures::never::Never,
    import::Importer,
    sqlx::{Pool, Sqlite},
    std::{path::PathBuf, sync::Arc},
    texthook::PullTexthooker,
    tokio::sync::broadcast,
    wordbase::{DictionaryState, ProfileId, ProfileState, hook::HookSentence},
};

#[derive(Debug, Clone, Deref, DerefMut)]
#[non_exhaustive]
pub struct Engine(Arc<Inner>);

#[derive(Debug)]
pub struct Inner {
    db: Pool<Sqlite>,
    importer: Importer,
    pull_texthooker: PullTexthooker,
    send_event: broadcast::Sender<Event>,
}

impl Engine {
    pub async fn new(
        config: &Config,
    ) -> Result<(Self, impl Future<Output = Result<Never>> + use<>)> {
        let db = db::setup(&config.db_path, config.max_db_connections)
            .await
            .context("failed to set up database")?;
        let (send_event, _) = broadcast::channel(CHANNEL_BUF_CAP);
        let (pull_texthooker, pull_texthooker_task) = PullTexthooker::new(send_event.clone());

        let engine = Self(Arc::new(Inner {
            db,
            importer: Importer::new(),
            pull_texthooker,
            send_event,
        }));
        Ok((engine.clone(), async move {
            engine
                .set_texthooker_url(
                    engine
                        .texthooker_url()
                        .await
                        .context("failed to read initial texthooker url")?,
                )
                .await
                .context("failed to set initial texthooker url")?;
            pull_texthooker_task.await
        }))
    }

    #[must_use]
    pub fn recv_event(&self) -> broadcast::Receiver<Event> {
        self.send_event.subscribe()
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub max_db_connections: u32,
    pub max_concurrent_imports: usize,
}

#[derive(Debug, Clone)]
pub enum Event {
    ProfileAdded { profile: ProfileState },
    ProfileRemoved { profile_id: ProfileId },
    PullTexthookerConnected,
    PullTexthookerDisconnected,
    HookSentence(HookSentence),
    SyncDictionaries(Vec<DictionaryState>),
}

const CHANNEL_BUF_CAP: usize = 4;
