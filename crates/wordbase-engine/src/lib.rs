#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

pub mod anki;
mod db;
pub mod deinflect;
pub mod dictionary;
pub mod import;
pub mod lang;
pub mod lookup;
pub mod profile;
pub mod texthook;

pub use wordbase;
use wordbase::{DictionaryId, NormString};
use {
    anki::Anki,
    anyhow::{Context, Result},
    arc_swap::ArcSwap,
    deinflect::Deinflectors,
    derive_more::{Deref, DerefMut, Display, Error},
    dictionary::Dictionaries,
    directories::ProjectDirs,
    import::Imports,
    profile::Profiles,
    sqlx::{Pool, Sqlite},
    std::{
        path::{Path, PathBuf},
        sync::Arc,
    },
    texthook::Texthookers,
    tokio::sync::broadcast,
    tracing::info,
    wordbase::{ProfileId, TexthookerSentence},
};

#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Engine(Arc<Inner>);

#[derive(Debug)]
pub struct Inner {
    profiles: ArcSwap<Profiles>,
    dictionaries: ArcSwap<Dictionaries>,
    texthookers: Texthookers,
    imports: Imports,
    deinflectors: Deinflectors,
    anki: Anki,
    send_event: broadcast::Sender<EngineEvent>,
    db: Pool<Sqlite>,
}

#[derive(Debug, Clone)]
pub enum EngineEvent {
    ProfileAdded {
        id: ProfileId,
    },
    ProfileCopied {
        src_id: ProfileId,
        new_id: ProfileId,
    },
    ProfileRemoved {
        id: ProfileId,
    },
    ProfileNameSet {
        id: ProfileId,
    },
    FontFamilySet {
        profile_id: ProfileId,
    },
    DictionaryAdded {
        id: DictionaryId,
    },
    DictionaryRemoved {
        id: DictionaryId,
    },
    DictionaryPositionsSwapped {
        a_id: DictionaryId,
        b_id: DictionaryId,
    },
    DictionaryEnabled {
        profile_id: ProfileId,
        dictionary_id: DictionaryId,
    },
    DictionaryDisabled {
        profile_id: ProfileId,
        dictionary_id: DictionaryId,
    },
    SortingDictionarySet {
        profile_id: ProfileId,
        dictionary_id: Option<DictionaryId>,
    },
    AnkiDeckSet {
        profile_id: ProfileId,
    },
    AnkiNoteTypeSet {
        profile_id: ProfileId,
    },
    PullTexthookerConnected,
    PullTexthookerDisconnected,
    TexthookerSentence(TexthookerSentence),
}

pub type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;
pub type IndexSet<T> = indexmap::IndexSet<T, foldhash::fast::RandomState>;

impl Engine {
    pub async fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref();
        info!("Creating engine using {data_dir:?} as data directory");

        let db_path = data_dir.join("wordbase.db");
        let db = db::setup(&db_path).await?;
        let (send_event, _) = broadcast::channel(CHANNEL_BUF_CAP);
        let engine = Self(Arc::new(Inner {
            profiles: ArcSwap::from_pointee(
                Profiles::fetch(&db)
                    .await
                    .context("failed to fetch initial profiles")?,
            ),
            dictionaries: ArcSwap::from_pointee(
                Dictionaries::fetch(&db)
                    .await
                    .context("failed to fetch initial dictionaries")?,
            ),
            texthookers: Texthookers::new(&db, send_event.clone())
                .await
                .context("failed to create texthooker listener")?,
            imports: Imports::new(),
            deinflectors: Deinflectors::new().context("failed to create deinflectors")?,
            anki: Anki::new(&db)
                .await
                .context("failed to create Anki integration")?,
            send_event,
            db,
        }));
        Ok(engine)
    }

    #[must_use]
    pub fn recv_event(&self) -> broadcast::Receiver<EngineEvent> {
        self.send_event.subscribe()
    }
}

pub fn data_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("io.github", "aecsocket", "Wordbase")
        .context("failed to get default app directories")?;
    Ok(dirs.data_dir().to_path_buf())
}

#[derive(Debug, Clone, Display, Error)]
#[display("not found")]
pub struct NotFound;

const CHANNEL_BUF_CAP: usize = 4;
