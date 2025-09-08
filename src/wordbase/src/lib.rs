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

pub use wordbase_api::*;
use {
    anyhow::{Context, Result},
    arc_swap::ArcSwap,
    deinflect::Deinflectors,
    derive_more::{Display, Error},
    dictionary::Dictionaries,
    profile::Profiles,
    render::Renderer,
    sqlx::{Pool, Sqlite},
    std::{path::Path, sync::Arc, time::Instant},
    tokio::fs,
    tracing::{info, trace},
};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

#[derive(Debug)]
pub struct Engine {
    profiles: ArcSwap<Profiles>,
    dictionaries: Arc<ArcSwap<Dictionaries>>,
    renderer: Renderer,
    deinflectors: Deinflectors,
    db: Pool<Sqlite>,
}

pub type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;
pub type IndexSet<T> = indexmap::IndexSet<T, foldhash::fast::RandomState>;

impl Engine {
    pub async fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref();
        info!("Creating engine using {data_dir:?} as data directory");

        let start = Instant::now();
        let (db, ()) = tokio::join!(
            async {
                fs::create_dir_all(data_dir)
                    .await
                    .context("failed to create data directory")?;
                let db_path = data_dir.join("wordbase.db");
                let db = db::setup(&db_path).await;
                trace!("[{:?}] Setup database", start.elapsed());
                db
            },
            async {
                jmdict_furigana::init().await;
                trace!("[{:?}] Initialized `jmdict_furigana`", start.elapsed());
            }
        );
        let db = db?;

        let (profiles, dictionaries, renderer, deinflectors) = tokio::try_join!(
            async {
                let profiles = Profiles::fetch(&db)
                    .await
                    .context("failed to fetch initial profiles")?;
                trace!("[{:?}] Fetched profiles", start.elapsed());
                anyhow::Ok(profiles)
            },
            async {
                let dictionaries = Dictionaries::fetch(&db)
                    .await
                    .context("failed to fetch initial dictionaries")?;
                trace!("[{:?}] Fetched dictionaries", start.elapsed());
                anyhow::Ok(dictionaries)
            },
            async {
                let renderer = tokio::task::spawn_blocking(Renderer::new)
                    .await
                    .context("failed to create renderer")??;
                trace!("[{:?}] Created renderer", start.elapsed());
                anyhow::Ok(renderer)
            },
            async {
                let deinflectors = tokio::task::spawn_blocking(Deinflectors::new)
                    .await
                    .context("failed to create deinflectors")??;
                trace!("[{:?}] Created deinflectors", start.elapsed());
                anyhow::Ok(deinflectors)
            },
        )?;

        Ok(Self {
            profiles: ArcSwap::from_pointee(profiles),
            dictionaries: Arc::new(ArcSwap::from_pointee(dictionaries)),
            renderer,
            deinflectors,
            db,
        })
    }
}

#[derive(Debug, Clone, Display, Error)]
#[display("not found")]
pub struct NotFound;

pub const CHANNEL_BUF_CAP: usize = 4;

#[cfg(feature = "uniffi")]
mod ffi {
    use {
        crate::Engine,
        derive_more::{Display, Error, From},
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
