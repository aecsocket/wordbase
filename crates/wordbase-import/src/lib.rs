#![doc = include_str!("../README.md")]

mod yomichan_audio;
mod yomitan;
pub use {yomichan_audio::YomichanAudio, yomitan::Yomitan};

/// All known importers from this crate.
pub const IMPORTERS: &[&'static dyn wordbase_storage::import::StartImport] =
    &[&Yomitan, &YomichanAudio];
