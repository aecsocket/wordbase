#![doc = include_str!("../README.md")]

use wordbase::{DictionaryState, hook::HookSentence};

mod db;
mod import;
mod texthooker;

#[derive(Debug, Clone)]
pub enum Event {
    HookSentence(HookSentence),
    SyncDictionaries(Vec<DictionaryState>),
}

pub const CHANNEL_BUF_CAP: usize = 4;
