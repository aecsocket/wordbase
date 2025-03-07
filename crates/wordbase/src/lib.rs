#![doc = include_str!("../README.md")]

pub mod lang;
pub mod lookup;
pub mod protocol;
mod ty;
pub(crate) mod util;
#[cfg(feature = "yomitan")]
pub mod yomitan;

pub use ty::*;

/// Default port which a Wordbase server listens on.
pub const DEFAULT_PORT: u16 = 9518;
