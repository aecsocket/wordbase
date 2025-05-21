//! Shared types for Japanese dictionaries.

use serde::{Deserialize, Serialize};

/// [Mora][mora] position in a word.
///
/// [mora]: https://en.wikipedia.org/wiki/Mora_(linguistics)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PitchPosition(pub u64);

#[cfg(feature = "uniffi")]
uniffi::custom_newtype!(PitchPosition, u64);
