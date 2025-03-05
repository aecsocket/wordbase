#![doc = include_str!("../README.md")]

pub mod protocol;
pub mod schema;

#[cfg(feature = "yomitan")]
pub mod yomitan;

use serde::{Deserialize, Serialize};

/// Default port which a Wordbase server listens on.
pub const DEFAULT_PORT: u16 = 9518;

/// Configuration shared between a Wordbase client and server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedConfig {
    /// Maximum length, in **characters** (not bytes), that [`Lookup::text`] is
    /// allowed to be.
    ///
    /// The maximum length of lookup requests is capped to avoid overloading the
    /// server with extremely large lookup requests. Clients must respect the
    /// server's configuration and not send any lookups longer than this,
    /// otherwise the server must return an error.
    ///
    /// [`Lookup::text`]: protocol::Lookup::text
    pub max_lookup_len: u16,
}

impl Default for SharedConfig {
    fn default() -> Self {
        Self { max_lookup_len: 16 }
    }
}
