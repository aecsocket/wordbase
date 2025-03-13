#![doc = include_str!("../README.md")]

mod db;
pub mod import;
pub mod popup;
mod texthooker;

use std::{num::NonZero, sync::Arc};

use import::Imports;
use popup::Popups;
use serde::{Deserialize, Serialize};
use wordbase::{DictionaryState, ProfileId, hook::HookSentence};

pub struct Server {
    pub imports: Imports,
}

impl Server {
    pub fn new(popups: Arc<dyn Popups>) -> Self {
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
struct Config {
    #[serde(skip)]
    current_profile: ProfileId,
    #[serde(default = "default_max_db_connections")]
    max_db_connections: NonZero<u32>,
    #[serde(default = "default_max_concurrent_imports")]
    max_concurrent_imports: NonZero<u32>,
    #[serde(default = "default_texthooker_sources")]
    texthooker_sources: Vec<TexthookerSource>,
}

fn default_max_db_connections() -> NonZero<u32> {
    NonZero::new(8).expect("should be greater than 0")
}

fn default_max_concurrent_imports() -> NonZero<u32> {
    NonZero::new(4).expect("should be greater than 0")
}

fn default_texthooker_sources() -> Vec<TexthookerSource> {
    vec![TexthookerSource {
        url: "ws://127.0.0.1:9001".into(),
    }]
}
