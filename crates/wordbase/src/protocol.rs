//! Types defining the messages exchanged between a client and server over a
//! WebSocket connection.

use derive_more::{Display, Error, From};
use serde::{Deserialize, Serialize};

use crate::{Dictionary, DictionaryId, LookupConfig, Record, Term, hook::HookSentence};

/// Default port which a Wordbase server listens on.
pub const DEFAULT_PORT: u16 = 9518;

/// Client-to-server WebSocket message, encoded as JSON.
#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum FromClient {
    /// See [`HookSentence`].
    #[from]
    HookSentence(HookSentence),
    /// Requests the server to find the first [terms] in some text, and return
    /// [records] for those terms.
    ///
    /// Server responds with 0 to N [`FromServer::Lookup`]s, ending with a final
    /// [`FromServer::LookupDone`].
    ///
    /// [records]: crate::Record
    /// [terms]: crate::Term
    Lookup {
        /// Text to search in.
        ///
        /// This must not be longer **in characters** than
        /// [`LookupConfig::max_request_len`].
        text: String,
    },
    /// Requests to remove a [dictionary] from the server's database.
    ///
    /// Server responds with [`FromServer::RemoveDictionary`].
    ///
    /// [dictionary]: Dictionary
    RemoveDictionary {
        /// ID of the dictionary.
        dictionary_id: DictionaryId,
    },
    /// Requests to [enable or disable][enabled] a [dictionary] in the server's
    /// database.
    ///
    /// Server responds with [`FromServer::SetDictionaryEnabled`].
    ///
    /// [enabled]: Dictionary::enabled
    /// [dictionary]: Dictionary
    SetDictionaryEnabled {
        /// ID of the dictionary.
        dictionary_id: DictionaryId,
        /// What [`Dictionary::enabled`] should be set to.
        enabled: bool,
    },
}

/// Server-to-client WebSocket message, encoded as JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FromServer {
    /// An unknown error occurred.
    Error {
        /// Arbitrary error message string.
        message: String,
    },
    /// Server sends its current [`LookupConfig`] to the client.
    SyncLookupConfig {
        /// Configuration.
        lookup_config: LookupConfig,
    },
    /// Server sends its current [`Dictionary`] list to the client.
    ///
    /// This is sent when dictionaries are modified - added, removed, etc.
    SyncDictionaries {
        /// Dictionaries.
        dictionaries: Vec<Dictionary>,
    },
    /// See [`HookSentence`].
    HookSentence(HookSentence),
    Lookup {
        record: RecordLookup,
    },
    LookupDone,
    RemoveDictionary {
        result: Result<(), DictionaryNotFound>,
    },
    SetDictionaryEnabled {
        result: Result<(), DictionaryNotFound>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordLookup {
    pub source: DictionaryId,
    pub term: Term,
    pub record: Record,
}

/// Attempted to perform an operation on a [`DictionaryId`] which does not
/// exist.
#[derive(Debug, Clone, Display, Error, Serialize, Deserialize)]
#[display("dictionary not found")]
pub struct DictionaryNotFound;

#[cfg(test)]
mod tests {
    use serde::de::DeserializeOwned;

    use super::*;

    fn default<T: Default>() -> T {
        T::default()
    }

    fn round_trip<T: Serialize + DeserializeOwned>(original: T) {
        let json = serde_json::to_string_pretty(&original).unwrap();
        println!("{json}");
        serde_json::from_str::<T>(&json).unwrap();
    }

    #[test]
    fn round_trip_all() {
        round_trip(FromClient::from(HookSentence::default()));
        round_trip(FromClient::Lookup { text: default() });
        round_trip(FromClient::RemoveDictionary {
            dictionary_id: default(),
        });
        round_trip(FromClient::SetDictionaryEnabled {
            dictionary_id: default(),
            enabled: default(),
        });

        round_trip(FromServer::Error { message: default() });
        round_trip(FromServer::SyncLookupConfig {
            lookup_config: default(),
        });
        round_trip(FromServer::SyncDictionaries {
            dictionaries: vec![default()],
        });
        round_trip(FromServer::HookSentence(HookSentence::default()));
        round_trip(FromServer::Lookup {
            record: RecordLookup {
                source: default(),
                term: default(),
                record: Record::Glossary(default()),
            },
        });
        round_trip(FromServer::LookupDone);
        round_trip(FromServer::RemoveDictionary {
            result: Err(DictionaryNotFound),
        });
        round_trip(FromServer::SetDictionaryEnabled { result: Ok(()) });
        round_trip(FromServer::SetDictionaryEnabled {
            result: Err(DictionaryNotFound),
        });
    }
}
