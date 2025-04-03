//! Types defining the messages exchanged between a client and server over a
//! WebSocket connection.

use {
    crate::{DictionaryId, Record, RecordKind, Term, hook::HookSentence},
    derive_more::{Display, Error, From},
    serde::{Deserialize, Serialize},
};

/// Default WebSocket port which a Wordbase server listens on.
pub const DEFAULT_WS_PORT: u16 = 9518;

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
    /// See [`HookSentence`].
    #[from]
    HookSentence(HookSentence),
    /// Server sends a response to [`FromClient::Lookup`] containing a single
    /// record.
    #[from]
    Lookup(RecordLookup),
    /// Server sends a response to [`FromClient::Lookup`] marking that all
    /// records have been sent.
    LookupDone,
    /// Server sends a response to [`FromClient::ShowPopup`].
    ShowPopup {
        /// Whether showing the popup was successful.
        result: Result<ShowPopupResponse, ShowPopupError>,
    },
    /// Server sends a response to [`FromClient::HidePopup`] marking success.
    HidePopup,
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
    /// This may be arbitrarily large, but the server may limit how far ahead it
    /// reads to find lookup results.
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
pub struct RecordLookup {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowPopupRequest {
    /// Specifies what window the popup will be positioned relative to.
    ///
    /// If no windows match this filter, or more than 1 window matches, then the
    /// popup request will fail. Therefore, for the best chance of success, try
    /// to be as specific as possible
    pub target_window: WindowFilter,
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

/// Specifies a specific window on the user's window manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowFilter {
    /// Internal ID of the window.
    ///
    /// This is an opaque identifier which is entirely platform-specific.
    /// This is the most reliable identifier to use to identify a window, but
    /// is usually internal to the window manager. If you have access to this
    /// ID, you should specify only this.
    ///
    /// # Platforms
    ///
    /// - Linux/Wayland
    ///   - GNOME: [`Meta / Window: get_id()`][gnome] (accessible if you are
    ///     writing a shell extension)
    ///
    /// [gnome]: https://gjs-docs.gnome.org/meta16~16/meta.window#method-get_id
    pub id: Option<u64>,
    /// Process ID which owns the target window.
    pub pid: Option<u32>,
    /// Title of the target window.
    pub title: Option<String>,
    /// Linux `WM_CLASS` (or whatever is reported as the `WM_CLASS`) of the
    /// target window.
    pub wm_class: Option<String>,
}

/// What corner a [`ShowPopupRequest`] is relative to.
///
/// See [`ShowPopupRequest::anchor`].
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

/// Popup was shown after a [`ShowPopupRequest`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowPopupResponse {
    /// Number of **bytes** (not characters) along the text that were scanned,
    /// in order to look up records for them.
    ///
    /// You can use this to e.g. highlight the text that is being looked up.
    pub scan_len: u64,
}

/// Failed to show a popup using [`ShowPopupRequest`].
#[derive(Debug, Clone, Copy, Display, Error, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ShowPopupError {
    /// There were no records to show in the popup.
    #[display("no records to show")]
    NoRecords,
}

/// Attempted to perform an operation on an entity which does not exist.
#[derive(Debug, Clone, Copy, Default, Display, Error, Serialize, Deserialize)]
#[display("not found")]
pub struct NotFound;

#[cfg(test)]
mod tests {
    use {super::*, serde::de::DeserializeOwned};

    fn default<T: Default>() -> T {
        T::default()
    }

    #[expect(clippy::needless_pass_by_value, reason = "improves ergonomics")]
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

        round_trip(FromServer::Error { message: default() });
        round_trip(FromServer::from(HookSentence::default()));
        round_trip(FromServer::from(RecordLookup {
            source: DictionaryId(default()),
            term: Term::new(""),
            record: Record::GlossaryHtml(default()),
        }));
        round_trip(FromServer::LookupDone);
    }
}
