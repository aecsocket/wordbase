#![doc = include_str!("../README.md")]

mod yomichan_audio;
mod yomitan;
pub use {wordbase_storage_api::import::*, yomichan_audio::YomichanAudio, yomitan::Yomitan};

/// All known importers from this crate.
pub const IMPORTERS: &[&dyn Importer] = &[&Yomitan, &YomichanAudio];
