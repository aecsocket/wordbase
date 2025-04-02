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
pub mod texthook;

pub use wordbase;
use {
    anyhow::Result,
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
    db: Pool<Sqlite>,
    imports: Imports,
    texthookers: Texthookers,
}

impl Engine {
    pub async fn new(db_path: impl AsRef<Path>) -> Result<Self> {
        let db = db::setup(db_path.as_ref()).await?;
        let engine = Self(Arc::new(Inner {
            db,
            imports: Imports::new(),
            texthookers: Texthookers::new(),
        }));
        Ok(engine)
    }
}

const CHANNEL_BUF_CAP: usize = 4;
