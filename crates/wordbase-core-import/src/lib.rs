#![doc = include_str!("../README.md")]

mod yomichan_audio;
mod yomitan;

use wordbase_core::importer::Importer;
pub use {yomichan_audio::YomichanAudio, yomitan::Yomitan};

/// All [`Importer`]s from this crate.
pub const IMPORTERS: &[&dyn Importer] = &[
    #[cfg(feature = "importer-yomitan")]
    &Yomitan,
    #[cfg(feature = "importer-yomichan-audio")]
    &YomichanAudio,
];
