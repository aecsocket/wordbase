#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

// pub mod anki;
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
    derive_more::{Display, Error},
    dictionary::Dictionaries,
    profile::Profiles,
    sqlx::{Pool, Sqlite},
    std::{path::Path, sync::Arc},
    tokio::fs,
    tracing::info,
};

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();

#[derive(Debug)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct Wordbase {
    db: Pool<Sqlite>,
    profiles: ArcSwap<Profiles>,
    dictionaries: Arc<ArcSwap<Dictionaries>>,
}

pub type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;
pub type IndexSet<T> = indexmap::IndexSet<T, foldhash::fast::RandomState>;

impl Wordbase {
    pub async fn new(data_dir: impl AsRef<Path>) -> Result<Self> {
        let data_dir = data_dir.as_ref();
        info!("Creating engine using {data_dir:?} as data directory");

        fs::create_dir_all(data_dir)
            .await
            .context("failed to create data directory")?;
        let db_path = data_dir.join("wordbase.db");
        let db = db::setup(&db_path)
            .await
            .context("failed to setup database")?;

        let (profiles, dictionaries) = tokio::try_join!(
            async {
                Profiles::fetch(&db)
                    .await
                    .context("failed to fetch initial profiles")
            },
            async {
                Dictionaries::fetch(&db)
                    .await
                    .context("failed to fetch initial dictionaries")
            },
        )?;

        Ok(Self {
            db,
            profiles: ArcSwap::from_pointee(profiles),
            dictionaries: Arc::new(ArcSwap::from_pointee(dictionaries)),
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
        crate::Wordbase,
        derive_more::{Display, Error, From},
    };

    #[derive(Debug, Display, Error, From, uniffi::Error)]
    #[uniffi(flat_error)]
    pub enum WordbaseError {
        #[display("{_0:?}")]
        Ffi(anyhow::Error),
    }

    pub type FfiResult<T> = Result<T, WordbaseError>;

    #[uniffi::export(async_runtime = "tokio")]
    pub async fn wordbase(data_dir: &str) -> FfiResult<Wordbase> {
        Ok(Wordbase::new(data_dir).await?)
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
        Wordbase::new(&data_path).await.unwrap();
    }
}
