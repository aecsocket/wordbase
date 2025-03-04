#![doc = include_str!("../README.md")]

pub mod lookup;
pub mod protocol;

#[cfg(feature = "yomitan")]
pub mod yomitan;

/// Default port which a Wordbase server listens on.
pub const DEFAULT_PORT: u16 = 9518;
