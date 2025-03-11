//! Types defining the messages exchanged between a client and server over a
//! WebSocket connection.

use {
    crate::{DictionaryId, DictionaryState, Record, RecordKind, Term, hook::HookSentence},
    derive_more::{Display, Error, From},
    serde::{Deserialize, Serialize},
};

/// Default port which a Wordbase server listens on.
pub const DEFAULT_PORT: u16 = 9518;

/// Client-to-server WebSocket message, encoded as JSON.
#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum FromClient {
    /// See [`HookSentence`].
    #[from]
    HookSentence(HookSentence),
    /// See [`LookupRequest`].
    #[from]
    Lookup(LookupRequest),
    /// See [`ShowPopupRequest`].
    #[from]
    ShowPopup(ShowPopupRequest),
    /// Requests to hide the currently shown popup dictionary.
    ///
    /// Server responds with [`FromServer::HidePopup`].
    HidePopup,
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
    /// Requests to set the [position] of a [dictionary] used for sorting lookup
    /// records.
    ///
    /// If a dictionary already exists at the given position, both dictionaries
    /// will share the same position, and record lookup order will be
    /// non-deterministic between the two dictionaries.
    ///
    /// Server responds with [`FromServer::SetDictionaryPosition`].
    ///
    /// [position]: Dictionary::position
    /// [dictionary]: Dictionary
    SetDictionaryPosition {
        /// ID of the dictionary.
        dictionary_id: DictionaryId,
        /// New position of the dictionary.
        position: i64,
    },
}

/// Server-to-client WebSocket message, encoded as JSON.
#[derive(Debug, Clone, Serialize, Deserialize, From)]
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
        dictionaries: Vec<DictionaryState>,
    },
    /// See [`HookSentence`].
    #[from]
    HookSentence(HookSentence),
    /// Server sends a response to [`FromClient::Lookup`] containing a single
    /// record.
    #[from]
    Lookup(LookupResponse),
    /// Server sends a response to [`FromClient::ShowPopup`].
    ShowPopup {
        /// Whether showing the popup was successful.
        result: Result<ShowPopupResponse, NoRecords>,
    },
    /// Server sends a response to [`FromClient::HidePopup`] marking success.
    HidePopup,
    /// Server sends a response to [`FromClient::Lookup`] marking that all
    /// records have been sent.
    LookupDone,
    /// Response to [`FromClient::RemoveDictionary`].
    RemoveDictionary {
        /// Result of the operation.
        result: Result<(), DictionaryNotFound>,
    },
    /// Response to [`FromClient::SetDictionaryEnabled`].
    SetDictionaryEnabled {
        /// Result of the operation.
        result: Result<(), DictionaryNotFound>,
    },
    /// Response to [`FromClient::SetDictionaryPosition`].
    SetDictionaryPosition {
        /// Result of the operation.
        result: Result<(), DictionaryNotFound>,
    },
}

/// Configuration for [lookup operations] shared between a Wordbase client and
/// server.
///
/// [lookup operations]: protocol::FromClient::Lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupConfig {
    /// Maximum length, in **characters** (not bytes), that [`Lookup::text`] is
    /// allowed to be.
    ///
    /// The maximum length of lookup requests is capped to avoid overloading the
    /// server with extremely large lookup requests. Clients must respect the
    /// server's configuration and not send any lookups longer than this,
    /// otherwise the server will return an error.
    ///
    /// [`Lookup::text`]: protocol::FromClient::Lookup::text
    pub max_request_len: u64,
}

impl Default for LookupConfig {
    fn default() -> Self {
        Self {
            max_request_len: 16,
        }
    }
}

/// Requests the server to find the first [terms] in some text, and return
/// [records] for those terms.
///
/// Server responds with 0 to N [`FromServer::Lookup`]s, ending with a final
/// [`FromServer::LookupDone`].
///
/// [records]: crate::Record
/// [terms]: crate::Term
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LookupRequest {
    /// Text to search in.
    ///
    /// This must not be longer **in characters** than
    /// [`LookupConfig::max_request_len`].
    pub text: String,
    /// What kinds of records the server should send us.
    ///
    /// Clients must explicitly list what kinds of records they want to receive,
    /// as it is possible (and expected!) that clients won't be able to process
    /// all of them.
    pub record_kinds: Vec<RecordKind>,
}

/// Single record returned by the server in response to a [`LookupRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResponse {
    /// Canonical dictionary form of the term for which this record is for.
    pub lemma: String,
    /// ID of the [dictionary] from which the record was retrieved.
    ///
    /// [dictionary]: Dictionary
    pub source: DictionaryId,
    /// The [term] that this record is for.
    ///
    /// [term]: Term
    pub term: Term,
    /// The [record] that was found.
    ///
    /// [record]: Record
    pub record: Record,
}

/// Requests the server to create a dictionary popup window on top of a
/// target window, and show it on the user's window manager.
///
/// The popup will be positioned relative to a target window; however, you must
/// manually fill out the details of this target window. If no windows match,
/// or more than 1 window matches your filters, then the request will fail.
/// Therefore, for the best chance of success, try to fill out as many target
/// fields as possible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowPopupRequest {
    /// Internal ID of the window.
    ///
    /// This is an opaque identifier which is entirely platform-specific.
    /// This is the most reliable identifier to use to identify a window, but
    /// is usually internal to the window manager. If you are working in an
    /// environment where you have access to this ID (i.e. a window manager
    /// extension), prioritise using this filter.
    pub target_id: Option<u64>,
    /// Process ID which owns the target window.
    pub target_pid: Option<u32>,
    /// Title of the target window.
    pub target_title: Option<String>,
    /// Linux `WM_CLASS` (or whatever is reported as the `WM_CLASS`) of the
    /// target window.
    pub target_wm_class: Option<String>,
    /// X and Y position of the pop-up [origin], in surface-local coordinates.
    ///
    /// These coordinates are relative to the top-left of the target window's
    /// frame (what the user would consider the "window", minus any decoration).
    ///
    /// [origin]: ShowPopupRequest::anchor
    pub origin: (i32, i32),
    /// What corner the popup will expand out from.
    pub anchor: PopupAnchor,
    /// Text to look up in the dictionary.
    ///
    /// You don't need to do any lookups yourself - the server and popup will
    /// handle this themselves.
    pub text: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[expect(missing_docs, reason = "self-explanatory")]
pub enum PopupAnchor {
    TopLeft,
    TopMiddle,
    TopRight,
    MiddleLeft,
    MiddleRight,
    BottomLeft,
    BottomMiddle,
    BottomRight,
}

/// Popup was shown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowPopupResponse {
    /// Number of **characters** (not bytes) along the text that were scanned,
    /// in order to look up records for them.
    ///
    /// You can use this to e.g. highlight the text that is being looked up.
    pub chars_scanned: u64,
}

/// No records to show, therefore the popup was not shown.
#[derive(Debug, Clone, Copy, Display, Error, Serialize, Deserialize)]
#[display("no records to show")]
pub struct NoRecords;

/// Attempted to perform an operation on a [`DictionaryId`] which does not
/// exist.
#[derive(Debug, Clone, Copy, Default, Display, Error, Serialize, Deserialize)]
#[display("dictionary not found")]
pub struct DictionaryNotFound;

#[cfg(test)]
mod tests {
    use {super::*, serde::de::DeserializeOwned};

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
        round_trip(FromClient::from(LookupRequest {
            text: default(),
            record_kinds: vec![RecordKind::GlossaryHtml],
        }));
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
        round_trip(FromServer::from(HookSentence::default()));
        round_trip(FromServer::from(LookupResponse {
            lemma: default(),
            source: default(),
            term: default(),
            record: Record::GlossaryHtml(default()),
        }));
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
