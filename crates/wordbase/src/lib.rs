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
pub mod render;
#[cfg(feature = "desktop")]
pub mod texthook;

use tokio::fs;
pub use wordbase_api::*;
use {
    anyhow::{Context, Result},
    arc_swap::ArcSwap,
    deinflect::Deinflectors,
    derive_more::{Display, Error},
    dictionary::Dictionaries,
    profile::Profiles,
    sqlx::{Pool, Sqlite},
    std::path::Path,
    tera::Tera,
    tokio::sync::broadcast,
    tracing::info,
};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

#[derive(Debug)]
pub struct Engine {
    profiles: ArcSwap<Profiles>,
    dictionaries: ArcSwap<Dictionaries>,
    renderer: Tera,
    #[cfg(feature = "desktop")]
    texthookers: texthook::Texthookers,
    deinflectors: Deinflectors,
    event_tx: broadcast::Sender<EngineEvent>,
    db: Pool<Sqlite>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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
    TexthookerConnected,
    TexthookerDisconnected,
    Sentence(TexthookerSentence),
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
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
    #[expect(clippy::missing_panics_doc, reason = "shouldn't panic")]
    pub async fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref();
        info!("Creating engine using {data_dir:?} as data directory");

        let (db, ()) = tokio::join!(
            async {
                fs::create_dir_all(data_dir)
                    .await
                    .context("failed to create data directory")?;
                let db_path = data_dir.join("wordbase.db");
                db::setup(&db_path).await
            },
            jmdict_furigana::init(),
        );
        let db = db?;

        let (event_tx, _) = broadcast::channel(CHANNEL_BUF_CAP);
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
            renderer: {
                let mut tera = Tera::default();
                tera.add_raw_template("records.html", include_str!("records.html"))
                    .expect("template should be valid");
                tera
            },
            #[cfg(feature = "desktop")]
            texthookers: texthook::Texthookers::new(&db, event_tx.clone())
                .await
                .context("failed to create texthooker listener")?,
            deinflectors: Deinflectors::new().context("failed to create deinflectors")?,
            // anki: Anki::new(&db)
            //     .await
            //     .context("failed to create Anki integration")?,
            event_tx,
            db,
        })
    }

    #[must_use]
    pub fn event_rx(&self) -> broadcast::Receiver<EngineEvent> {
        self.event_tx.subscribe()
    }
}

#[derive(Debug, Clone, Display, Error)]
#[display("not found")]
pub struct NotFound;

const CHANNEL_BUF_CAP: usize = 4;

#[cfg(feature = "desktop")]
pub fn data_dir() -> Result<std::path::PathBuf> {
    let dirs = directories::ProjectDirs::from("io.github", "aecsocket", "Wordbase")
        .context("failed to get default app directories")?;
    Ok(dirs.data_dir().to_path_buf())
}

#[cfg(feature = "uniffi")]
mod ffi {
    use {
        crate::{Engine, EngineEvent},
        derive_more::{Display, Error, From},
        tokio::sync::{Mutex, broadcast},
    };

    #[derive(Debug, uniffi::Object)]
    pub struct Wordbase(pub Engine);

    #[derive(Debug, Display, Error, From, uniffi::Error)]
    #[uniffi(flat_error)]
    pub enum WordbaseError {
        #[display("{_0:?}")]
        Ffi(anyhow::Error),
    }

    pub type FfiResult<T> = Result<T, WordbaseError>;

    #[uniffi::export(async_runtime = "tokio")]
    pub async fn wordbase(data_dir: &str) -> FfiResult<Wordbase> {
        Ok(Engine::new(data_dir).await.map(Wordbase)?)
    }

    #[derive(uniffi::Object)]
    pub struct EngineEventReceiver(Mutex<broadcast::Receiver<EngineEvent>>);

    #[uniffi::export]
    impl Wordbase {
        pub fn event_rx(&self) -> EngineEventReceiver {
            EngineEventReceiver(Mutex::new(self.0.event_rx()))
        }
    }

    #[uniffi::export(async_runtime = "tokio")]
    impl EngineEventReceiver {
        pub async fn recv(&self) -> Option<EngineEvent> {
            self.0.lock().await.recv().await.ok()
        }
    }
}

#[cfg(feature = "uniffi")]
pub use ffi::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn create_data_dir_if_not_exists() {
        let data_dir = tempfile::tempdir().unwrap();
        let data_path = data_dir.path().to_path_buf();
        data_dir.close().unwrap();
        Engine::new(&data_path).await.unwrap();
    }
}
