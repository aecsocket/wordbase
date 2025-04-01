#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

// pub mod anki;
mod db;
mod deinflect;
mod dictionary;
pub mod html;
pub mod import;
pub mod lang;
mod lookup;
mod profile;
mod texthook;

pub use wordbase;
use {
    anyhow::{Context, Result},
    derive_more::{Deref, DerefMut},
    futures::never::Never,
    import::Imports,
    sqlx::{Pool, Sqlite},
    std::{path::Path, sync::Arc},
    texthook::PullTexthooker,
    tokio::sync::broadcast,
    wordbase::{DictionaryId, Profile, ProfileId, TexthookerSentence},
};

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Engine(Arc<Inner>);

#[derive(Debug)]
pub struct Inner {
    db: Pool<Sqlite>,
    imports: Imports,
    pull_texthooker: PullTexthooker,
    // anki: Anki,
    send_event: broadcast::Sender<Event>,
}

impl Engine {
    pub async fn new<P: AsRef<Path>>(
        db_path: P,
    ) -> Result<(Self, impl Future<Output = Result<Never>> + use<P>)> {
        let db = db::setup(db_path.as_ref())
            .await
            .context("failed to set up database")?;
        let (send_event, _) = broadcast::channel(CHANNEL_BUF_CAP);
        let (pull_texthooker, pull_texthooker_task) = PullTexthooker::new(send_event.clone());
        // let anki = Anki::new();

        let engine = Self(Arc::new(Inner {
            db,
            imports: Imports::new(),
            pull_texthooker,
            // anki,
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
pub enum Event {
    ProfileAdded { profile: Profile },
    ProfileRemoved { id: ProfileId },
    DictionaryPositionSet { id: DictionaryId, position: i64 },
    DictionaryRemoved { id: DictionaryId },
    PullTexthookerConnected,
    PullTexthookerDisconnected,
    TexthookerSentence(TexthookerSentence),
}

const CHANNEL_BUF_CAP: usize = 4;
