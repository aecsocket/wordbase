use {
    crate::{DictionaryId, FrequencyValue, Record, RecordId, Term},
    serde::{Deserialize, Serialize},
    std::ops::Range,
};

// /// Parameters for performing a lookup against the engine's database.
// ///
// /// Lookups return a stream of [`RecordLookup`]s. These lookups are
// pre-sorted /// by the engine based on relevance to the term you're looking
// up. #[derive(Debug, Clone, Default, Serialize, Deserialize)]
// pub struct Lookup {
//     /// Context text for the lookup.
//     ///
//     /// This should include the text you want to look up, as well as all the
//     /// surrounding content. For example, if looking up text in a video
// player's     /// subtitles, this can include the previous and next few
// subtitle lines; or     /// in a web browser, this could be the entire
// paragraph within which the     /// lookup text is contained.
//     ///
//     /// If you have no surrounding content, it's fine to only include the
// lookup     /// text in this field, but some actions (e.g. creating Anki
// cards) may have     /// less relevant information to work with.
//     pub context: String,
//     /// Byte index into [`Lookup::context`] which marks what text you
// actually     /// want to get lookup results for.
//     ///
//     /// The index must land on a UTF-8 character boundary.
//     ///
//     /// # Examples
//     ///
//     /// ```
//     /// let context = "the quick brown fox";
//     /// // we want to look up "brown"
//     /// let cursor = context.find("brown").unwrap();
//     /// ```
//     ///
//     /// ```
//     /// let context = "walk";
//     /// // we have no other context, so just...
//     /// let cursor = 0;
//     /// ```
//     pub cursor: usize,
// }

/// Single [`Record`] and its metadata returned in response to a lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct RecordEntry {
    /// Span in the lookup input sentence which corresponds to this entry's
    /// [`RecordEntry::term`], in UTF-8 string bytes.
    pub span_bytes: Span,
    /// Span in the lookup input sentence which corresponds to this entry's
    /// [`RecordEntry::term`], in Unicode characters.
    pub span_chars: Span,
    /// ID of the [`Dictionary`] from which the record was retrieved.
    ///
    /// [`Dictionary`]: crate::Dictionary
    pub source: DictionaryId,
    /// [`Term`] that this record is for.
    pub term: Term,
    /// ID of the [`Record`] that was found.
    pub record_id: RecordId,
    /// [`Record`] that was found.
    pub record: Record,
    /// [`FrequencyValue`] of the record, as found in the profile's sorting
    /// dictionary.
    pub profile_sorting_frequency: Option<FrequencyValue>,
    /// [`FrequencyValue`] of the record, as found in [`RecordEntry::source`]'s
    /// frequency information.
    pub source_sorting_frequency: Option<FrequencyValue>,
}

/// A (half-open) range bounded inclusively below and exclusively above
/// (`start..end`).
///
/// The range `start..end` contains all values with `start <= x < end`.
/// It is empty if `start >= end`.
///
/// We use this wrapper because [`Range`] is not supported by `poem-openapi`
/// or `uniffi`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Span {
    /// The lower bound of the range (inclusive).
    pub start: u64,
    /// The upper bound of the range (exclusive).
    pub end: u64,
}

impl<T: TryInto<u64>> TryFrom<Range<T>> for Span {
    type Error = T::Error;

    fn try_from(value: Range<T>) -> Result<Self, Self::Error> {
        Ok(Self {
            start: value.start.try_into()?,
            end: value.end.try_into()?,
        })
    }
}

// #[derive(Debug, Clone, Default, Serialize, Deserialize)]
// pub struct PopupRequest {
//     pub target_window: WindowFilter,
//     pub origin_nw: (i32, i32),
//     pub origin_se: (i32, i32),
//     pub lookup: Lookup,
// }

/// Specifies a specific window on the user's window manager.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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

/// Texthooker sentence event received from a [TextractorSender] server, in the
/// [exSTATic] format.
///
/// [TextractorSender]: https://github.com/KamWithK/TextractorSender
/// [exSTATic]: https://github.com/KamWithK/exSTATic/
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TexthookerSentence {
    /// Path of the process from which this texthooker sentence was extracted.
    ///
    /// This is not guaranteed to be in any format, but may be used as a
    /// persistent identifier.
    pub process_path: String,
    /// Extracted sentence.
    ///
    /// This may be malformed in some way, e.g. it may have trailing whitespace.
    pub sentence: String,
}

// pub enum Request {
//     Lookup {
//         lookup: Lookup,
//         /// What kinds of records we want to receive.
//         ///
//         /// You must explicitly list what kinds of records you want to
// receive,         /// as it is possible (and expected!) that you won't be able
// to         /// process all kinds of records.
//         ///
//         /// You can also use this to fetch a small amount of info when doing
// an         /// initial lookup, then fetch more records (e.g. pronunciation
// audio)         /// when the user selects a specific term.
//         record_kinds: Vec<RecordKind>,
//     },
// }
