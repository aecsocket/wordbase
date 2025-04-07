#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

// pub mod anki;
mod db;
pub mod deinflect;
pub mod dictionary;
pub mod html;
pub mod import;
pub mod lang;
pub mod lookup;
pub mod profile;
pub mod texthook;

use arc_swap::ArcSwap;
use dictionary::{Dictionaries, SharedDictionaries};
use profile::{Profiles, SharedProfiles};
use tokio::sync::broadcast;
pub use wordbase;
use wordbase::TexthookerSentence;
use {
    anyhow::{Context, Result},
    deinflect::Deinflectors,
    derive_more::{Deref, DerefMut},
    import::Imports,
    sqlx::{Pool, Sqlite},
    std::{path::Path, sync::Arc},
    texthook::Texthookers,
};

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Engine(Arc<Inner>);

#[derive(Debug)]
pub struct Inner {
    pub dictionaries: SharedDictionaries,
    pub profiles: SharedProfiles,
    pub texthookers: Texthookers,
    imports: Imports,
    deinflectors: Deinflectors,
    send_event: broadcast::Sender<Event>,
    db: Pool<Sqlite>,
}

#[derive(Debug, Clone)]
pub enum Event {
    SyncDictionaries,
    SyncProfiles,
    PullTexthookerConnected,
    PullTexthookerDisconnected,
    TexthookerSentence(TexthookerSentence),
}

pub type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

impl Engine {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db = db::setup(db_path.as_ref()).await?;
        let (send_event, _) = broadcast::channel(CHANNEL_BUF_CAP);
        let engine = Self(Arc::new(Inner {
            dictionaries: SharedDictionaries::new(ArcSwap::from_pointee(
                Dictionaries::fetch(&db)
                    .await
                    .context("failed to fetch initial dictionaries")?,
            )),
            profiles: SharedProfiles::new(ArcSwap::from_pointee(
                Profiles::fetch(&db)
                    .await
                    .context("failed to fetch initial profiles")?,
            )),
            texthookers: Texthookers::new(&db, send_event.clone())
                .await
                .context("failed to create texthooker listener")?,
            imports: Imports::new(),
            deinflectors: Deinflectors::new().context("failed to create deinflectors")?,
            send_event,
            db,
        }));
        Ok(engine)
    }

    #[must_use]
    pub fn recv_event(&self) -> broadcast::Receiver<Event> {
        self.send_event.subscribe()
    }
}

const CHANNEL_BUF_CAP: usize = 4;
