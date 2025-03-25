#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

mod db;
pub mod deinflect;
mod dictionary;
pub mod import;
mod lookup;
pub mod platform;
mod profile;

use {
    anyhow::{Context, Result},
    deinflect::Deinflectors,
    sqlx::{Pool, Sqlite},
    std::{path::PathBuf, sync::Arc},
    tokio::sync::{Mutex, Semaphore, broadcast},
    wordbase::{DictionaryState, ProfileId, ProfileState},
};

#[derive(Debug)]
#[non_exhaustive]
pub struct Engine {
    db: Pool<Sqlite>,
    send_event: broadcast::Sender<Event>,
    import_insert_lock: Arc<Mutex<()>>,
    pub import_concurrency: Arc<Semaphore>,
    pub recv_event: broadcast::Receiver<Event>,
    pub deinflectors: Deinflectors,
}

impl Clone for Engine {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            send_event: self.send_event.clone(),
            import_concurrency: self.import_concurrency.clone(),
            import_insert_lock: self.import_insert_lock.clone(),
            recv_event: self.recv_event.resubscribe(),
            deinflectors: self.deinflectors.clone(),
        }
    }
}

impl Engine {
    pub async fn new(config: &Config) -> Result<Self> {
        let db = db::setup(&config.db_path, config.max_db_connections)
            .await
            .context("failed to set up database")?;

        let (send_event, recv_event) = broadcast::channel(CHANNEL_BUF_CAP);
        Ok(Self {
            db,
            send_event,
            import_insert_lock: Arc::new(Mutex::new(())),
            import_concurrency: Arc::new(Semaphore::new(config.max_concurrent_imports)),
            recv_event,
            deinflectors: Deinflectors::default(),
        })
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
    SyncDictionaries(Vec<DictionaryState>),
}

const CHANNEL_BUF_CAP: usize = 4;
