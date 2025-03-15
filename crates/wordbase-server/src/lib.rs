#![doc = include_str!("../README.md")]

pub mod db;
pub mod import;
pub mod lookup;
pub mod popup;
pub mod texthooker;

use std::{num::NonZero, sync::Arc};

use import::Imports;
use lookup::Lookups;
use popup::Popups;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use wordbase::{DictionaryState, ProfileId, hook::HookSentence, protocol::LookupConfig};

#[derive(Debug)]
pub struct Server {
    pub imports: Imports,
    pub lookups: Lookups,
}

impl Server {
    pub fn new(db: Pool<Sqlite>, popups: Arc<dyn Popups>) -> Self {
        Self {
            imports: Imports::new(db, config),
        }
    }
}

pub const CHANNEL_BUF_CAP: usize = 4;

#[derive(Debug, Clone)]
pub enum Event {
    HookSentence(HookSentence),
    SyncDictionaries(Vec<DictionaryState>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TexthookerSource {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(skip)]
    pub current_profile: ProfileId,
    #[serde(default = "default_max_concurrent_imports")]
    pub max_concurrent_imports: NonZero<u32>,
    #[serde(default = "default_texthooker_sources")]
    pub texthooker_sources: Vec<TexthookerSource>,
    #[serde(default)]
    pub lookup: LookupConfig,
}

fn default_max_concurrent_imports() -> NonZero<u32> {
    NonZero::new(4).expect("should be greater than 0")
}

fn default_texthooker_sources() -> Vec<TexthookerSource> {
    vec![TexthookerSource {
        url: "ws://127.0.0.1:9001".into(),
    }]
}
