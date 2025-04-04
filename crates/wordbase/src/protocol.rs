use {
    crate::{DictionaryId, FrequencyValue, Record, RecordKind, Term},
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lookup {
    /// Context text for the lookup.
    ///
    /// This should include the text you want to look up, as well as all the
    /// surrounding content. For example, if looking up text in a video player's
    /// subtitles, this can include the previous and next few subtitle lines; or
    /// in a web browser, this could be the entire paragraph within which the
    /// lookup text is contained.
    ///
    /// If you have no surrounding content, it's fine to only include the lookup
    /// text in this field, but some actions (e.g. creating Anki cards) may be
    /// negatively impacted.
    pub context: String,
    /// Byte index into [`Lookup::context`] which marks what text you actually
    /// want to get lookup results for.
    ///
    /// # Examples
    ///
    /// ```
    /// let context = "the quick brown fox";
    /// // we want to look up "brown"
    /// let cursor = context.find("brown").unwrap();
    /// ```
    ///
    /// ```
    /// let context = "walk";
    /// // we have no other context, so just...
    /// let cursor = 0;
    /// ```
    pub cursor: usize,
    // TODO: image bytes? sentence audio? for anki cards
}

/// Single record returned in response to a [`Lookup`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordLookup {
    /// ID of the [`Dictionary`] from which the record was retrieved.
    pub source: DictionaryId,
    /// [`Term`] that this record is for.
    pub term: Term,
    /// [`Record`] that was found.
    pub record: Record,
    /// [`FrequencyValue`] of the associated record.
    pub frequency: Option<FrequencyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PopupRequest {
    pub target_window: WindowFilter,
    pub origin: (i32, i32),
    pub anchor: PopupAnchor,
    pub lookup: Lookup,
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
    TopCenter,
    TopRight,
    CenterLeft,
    CenterRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TexthookerSentence {
    pub process_path: String,
    pub sentence: String,
}

pub enum Request {
    Lookup {
        lookup: Lookup,
        /// What kinds of records we want to receive.
        ///
        /// You must explicitly list what kinds of records you want to receive,
        /// as it is possible (and expected!) that you won't be able to
        /// process all kinds of records.
        ///
        /// You can also use this to fetch a small amount of info when doing an
        /// initial lookup, then fetch more records (e.g. pronunciation audio)
        /// when the user selects a specific term.
        record_kinds: Vec<RecordKind>,
    },
}
