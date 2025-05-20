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
use wordbase::DictionaryId;
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
pub struct SharedEngine(Arc<Engine>);

#[derive(Debug)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct Engine {
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
    Profile(ProfileEvent),
    Dictionary(DictionaryEvent),
    FontFamilySet {
        profile_id: ProfileId,
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

#[derive(Debug, Clone)]
pub enum ProfileEvent {
    Added {
        id: ProfileId,
    },
    Copied {
        src_id: ProfileId,
        new_id: ProfileId,
    },
    Removed {
        id: ProfileId,
    },
    NameSet {
        id: ProfileId,
    },
}

#[derive(Debug, Clone)]
pub enum DictionaryEvent {
    Added {
        id: DictionaryId,
    },
    Removed {
        id: DictionaryId,
    },
    PositionsSwapped {
        a_id: DictionaryId,
        b_id: DictionaryId,
    },
    Enabled {
        profile_id: ProfileId,
        dictionary_id: DictionaryId,
    },
    Disabled {
        profile_id: ProfileId,
        dictionary_id: DictionaryId,
    },
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
        Ok(Self {
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
        })
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

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

#[cfg(feature = "uniffi")]
#[uniffi::export(async_runtime = "tokio")]
pub async fn engine(data_dir: &str) -> Engine {
    Engine::new(data_dir).await.unwrap()
}
