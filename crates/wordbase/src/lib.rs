#![doc = include_str!("../README.md")]

pub mod dict;

mod imp;
mod protocol;
use std::str::FromStr;

pub use protocol::*;

use {
    derive_more::{Debug, Deref, Display, Error, From},
    serde::{Deserialize, Serialize, de::DeserializeOwned},
};

/// Invokes a macro, passing in all existing dictionary and record kind into the
/// macro.
///
/// This serves as the source of truth for what dictionary and record kinds
/// exist in the current version of this crate. If you are adding a new kind,
/// add it here (documentation lives outside of this macro).
///
/// # Usage
///
/// Your macro will receive the following tokens:
///
/// ```text
/// $(
///     $dict_kind($dict_path) {
///         $( $record_kind ),*
///     }
/// ),*
/// ```
/// where:
/// - `$dict_kind` is:
///   - the `ident` of the [`DictionaryKind`] variant
///     - e.g. `Yomitan` maps to [`DictionaryKind::Yomitan`]
///   - the 1st half of [`RecordKind`] variant idents
///     - e.g. the `Yomitan` in [`RecordKind::YomitanGlossary`]
/// - `$dict_path` is the dictionary kind's `path` in [`dict`]
///   - e.g. `yomitan`
/// - `$record_kind` is:
///   - an `ident` of the type under `$dict_path`
///     - e.g. `Glossary` maps to `dict::yomitan::Glossary`
///   - the 2nd half of [`RecordKind`] variant idents
///     - e.g. the `Glossary` in [`RecordKind::YomitanGlossary`]
///
/// Trailing commas may be present in repetitions.
///
/// To form a [`DictionaryKind`] variant, you can use
/// `wordbase::DictionaryKind::$dict_kind`. To form a [`RecordKind`] variant,
/// you can combine `$dict_kind` and `$record_kind` via [`paste::paste`] like
/// so: `[< $dict_kind $record_kind >]`
///
/// # Examples
///
/// Generate top-level items for each record kind:
///
/// ```
/// macro_rules! define_types {
///     // copy this macro pattern exactly into your own macro
///     ($($dict_kind:ident($dict_path:ident) { $($record_kind:ident),* $(,)? }),* $(,)?) => {
///         // use `paste::paste` if you need to access record kinds
///         paste::paste! {
///             pub enum DictionaryKind {
///                 // single level of repetition here
///                 // to just iterate over the dictionary kinds
///                 $( $dict_kind, )*
///             }
///
///             pub enum RecordKind {
///                 // two levels of repetition here
///                 // to iterate over all record kinds
///                 $($( [< $dict_kind $record_kind >], )*)*
///             }
///         }
///     }
/// }
///
/// wordbase::for_kinds!(define_types);
/// ```
///
/// Generate code which performs the same action for all record kinds:
///
/// ```
/// # use wordbase::Record;
/// fn deserialize_record(kind: u32, data: &[u8]) {
///     macro_rules! deserialize_record { ($($dict_kind:ident($dict_path:ident) { $($record_kind:ident),* $(,)? }),* $(,)?) => { paste::paste! {{
///         mod discrim {
///             use wordbase::RecordKind;
///
///             $($(
///             pub const [< $dict_kind $record_kind >]: u32 = RecordKind::[< $dict_kind $record_kind >] as u32;
///             )*)*
///         }
///
///         match u32::try_from(kind) {
///             $($(
///             Ok(discrim::[< $dict_kind $record_kind >]) => {
///                 let record = deserialize(data);
///                 Record::[< $dict_kind $record_kind >](record)
///             }
///             )*)*
///             _ => panic!("invalid record kind {kind}"),
///         }
///     }}}}
///
///     wordbase::for_kinds!(deserialize_record);
/// }
/// # fn deserialize<T>(_: &[u8]) -> T { unimplemented!() }
/// ```
#[macro_export]
macro_rules! for_kinds { ($macro:ident) => { $macro!(
    Yomitan(yomitan) {
        Glossary,
        Frequency,
        Pitch,
    },
    YomichanAudio(yomichan_audio) {
        Forvo,
        Jpod,
        Nhk16,
        Shinmeikai8,
    },
); } }

macro_rules! define_types { ($($dict_kind:ident($dict_path:ident) { $($record_kind:ident),* $(,)? }),* $(,)?) => { paste::paste! {
/// Kind of [`Dictionary`] that can be imported into the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem", derive(poem_openapi::Enum))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[repr(u32)]
#[non_exhaustive]
#[expect(missing_docs, reason = "self-explanatory")]
pub enum DictionaryKind {
    $($dict_kind,)*
}

impl DictionaryKind {
    /// All variants of this enum.
    pub const ALL: &[Self] = &[$(Self::$dict_kind,)*];
}

/// Kind of [`RecordKind`] that a dictionary can contain, and that a client can
/// query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem", derive(poem_openapi::Enum))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[repr(u32)]
#[non_exhaustive]
#[expect(missing_docs, reason = "self-explanatory")]
pub enum RecordKind {
    $($([< $dict_kind $record_kind >],)*)*
}

impl RecordKind {
    /// All variants of this enum.
    pub const ALL: &[Self] = &[$($(Self::[< $dict_kind $record_kind >],)*)*];
}

/// Data that a [`Dictionary`] may store for a specific [`Term`].
#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
#[non_exhaustive]
#[expect(missing_docs, reason = "self-explanatory")]
pub enum Record {
    $($([< $dict_kind $record_kind >](dict::$dict_path::$record_kind),)*)*
}

impl Record {
    /// Gets the kind of this record.
    #[must_use]
    pub const fn kind(&self) -> RecordKind {
        match self {
            $($(Self::[< $dict_kind $record_kind >](_) => RecordKind::[< $dict_kind $record_kind >],)*)*
        }
    }
}

$($(
impl sealed::RecordType for dict::$dict_path::$record_kind {}

impl RecordType for dict::$dict_path::$record_kind {
    const KIND: RecordKind = RecordKind::[< $dict_kind $record_kind >];
}
)*)*
}}}
for_kinds!(define_types);

mod sealed {
    pub trait RecordType {}
}

/// Provides bounds on the type of data that can be stored in a [`Record`].
pub trait RecordType:
    sealed::RecordType
    + Sized
    + Send
    + Sync
    + std::fmt::Debug
    + Clone
    + Serialize
    + DeserializeOwned
    + Into<Record>
    + 'static
{
    /// [`RecordKind`] variant of this record type.
    const KIND: RecordKind;
}

/// Opaque and unique identifier for a [`Record`] in the engine.
///
/// Multiple [`Term`]s may link to a single [`Record`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem", derive(poem_openapi::NewType))]
pub struct RecordId(pub i64);

#[cfg(feature = "uniffi")]
uniffi::custom_newtype!(RecordId, i64);

/// Imported collection of [`Record`]s in the engine.
///
/// This represents a dictionary which has already been imported into the
/// engine, whereas [`DictionaryMeta`] may not.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "poem", derive(poem_openapi::Object))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[serde(deny_unknown_fields)]
pub struct Dictionary {
    /// Unique identifier for this dictionary in the database.
    pub id: DictionaryId,
    /// Meta information about this dictionary.
    pub meta: DictionaryMeta,
    /// What position [`Record`]s from this dictionary will be returned during
    /// [`Lookup`]s, relative to other dictionaries.
    ///
    /// A higher position means records from this dictionary will be returned
    /// later, and should be displayed to the user with a lower priority.
    pub position: i64,
}

/// Metadata for a [`Dictionary`].
///
/// This is `#[non_exhaustive]`: to create a new value, you must use
/// [`DictionaryMeta::new`] to create an initial value, then set fields
/// explicitly.
///
/// # Examples
///
/// ```
/// # use wordbase::DictionaryMeta;
///
/// let mut meta = DictionaryMeta::new(DictionaryKind::YomitanDictionary, "My Dictionary");
/// meta.version = Some("1.0.0".into());
/// meta.url = Some("https://example.com".into());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "poem", derive(poem_openapi::Object))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[non_exhaustive]
pub struct DictionaryMeta {
    /// What kind of dictionary this was imported from.
    pub kind: DictionaryKind,
    /// Human-readable display name.
    ///
    /// This value is **not guaranteed to be unique** across all dictionaries,
    /// however you may treat this as a stable identifier for a dictionary in
    /// its unimported form (i.e. the archive itself), and use this to detect if
    /// you attempt to import an already-imported dictionary.
    pub name: String,
    /// Arbitrary version string.
    ///
    /// This does not guarantee to conform to any format, e.g. semantic
    /// versioning.
    pub version: Option<String>,
    /// Describes the content of this dictionary.
    pub description: Option<String>,
    /// Homepage URL where users can learn more about this dictionary.
    pub url: Option<String>,
    /// Attribution information for the content of this dictionary.
    pub attribution: Option<String>,
}

/// Opaque and unique identifier for a [`Dictionary`] in the engine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem", derive(poem_openapi::NewType))]
pub struct DictionaryId(pub i64);

#[cfg(feature = "uniffi")]
uniffi::custom_newtype!(DictionaryId, i64);

/// Key for [`Record`]s in a [`Dictionary`].
///
/// A term consists of at least one of a headword or reading. Both the headword
/// and reading are guaranteed to be non-empty, enforced by [`NormString`].
///
/// For languages without the concept of a reading, only the headword should be
/// specified ([`Term::Headword`]), as this represents the canonical dictionary
/// form of this term.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Term {
    /// Has both a headword and a reading.
    #[debug("({headword:?}, {reading:?})")]
    #[display("{headword} ({reading})")]
    Full {
        /// Headword.
        headword: NormString,
        /// Reading.
        reading: NormString,
    },
    /// Has only a headword.
    #[debug("({headword:?}, ())")]
    #[display("{headword}")]
    Headword {
        /// Headword.
        headword: NormString,
    },
    /// Has only a reading.
    #[debug("((), {reading:?})")]
    #[display("{reading}")]
    Reading {
        /// Reading.
        reading: NormString,
    },
}

#[cfg(feature = "uniffi")]
const _: () = {
    #[derive(uniffi::Record)]
    pub struct TermFfi {
        headword: Option<NormString>,
        reading: Option<NormString>,
    }

    #[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Error)]
    #[display("no headword or reading")]
    pub struct NoHeadwordOrReading;

    uniffi::custom_type!(Term, TermFfi, {
        lower: |term| TermFfi {
            headword: term.headword().cloned(),
            reading: term.reading().cloned(),
        },
        try_lift: |ffi| Term::new(ffi.headword, ffi.reading).ok_or_else(|| NoHeadwordOrReading.into()),
    });
};

/// How often a [`Term`] appears in a single [`Dictionary`].
///
/// This value is used for sorting lookup results. However, the value given is
/// only valid in the context of a **single specific** [`Dictionary`]. That is,
/// if you take a [`FrequencyValue`] from one [`Dictionary`] and compare it to
/// another [`FrequencyValue`] from a different [`Dictionary`], the result is
/// meaningless.
///
/// There is explicitly no way to get the [`i64`] from this value while ignoring
/// the variant, as the value does not make sense without knowing if it's a rank
/// or an occurrence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum FrequencyValue {
    /// Lower value represents a [`Term`] which appears more frequently.
    Rank(i64),
    /// Lower value represents a [`Term`] which appears less frequently.
    Occurrence(i64),
}

/// Collection of user-defined settings which can be freely switched between.
///
/// The engine does not have a concept of a current profile - instead, it is the
/// app's responsibility to manage a current profile, and pass that profile ID
/// into operations which require it (e.g. lookups).
///
/// This represents a profile which already exists in the engine, whereas
/// [`ProfileMeta`] may not.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "poem", derive(poem_openapi::Object))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[non_exhaustive]
pub struct Profile {
    /// Unique identifier for this profile in the database.
    pub id: ProfileId,
    /// Name of the profile.
    ///
    /// User-defined profiles will always have a name. If the name is missing,
    /// then this is the default profile made on startup, and should be labelled
    /// to the user as "Default Profile" or similar.
    pub name: Option<NormString>,
    /// Which [`Dictionary`] is used for sorting records by their frequencies.
    ///
    /// The user-set dictionary [position] always takes priority over any
    /// frequency sorting.
    ///
    /// [position]: Dictionary::position
    pub sorting_dictionary: Option<DictionaryId>,
    /// System font family to use for text under this profile.
    ///
    /// This is *only* the family, e.g. `Adwaita Sans`, not the face like
    /// `Adwaita Sans Regular`.
    pub font_family: Option<String>,
    /// Name of the Anki deck used for AnkiConnect integration.
    pub anki_deck: Option<String>,
    /// Name of the Anki note type used for creating new notes.
    pub anki_note_type: Option<String>,
    /// Set of [`Dictionary`] entries which are [enabled] under this profile.
    ///
    /// [enabled]: Dictionary::enabled
    pub enabled_dictionaries: Vec<DictionaryId>,
}

/// Opaque and unique identifier for a [`Profile`] in the engine.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem", derive(poem_openapi::NewType))]
pub struct ProfileId(pub i64);

#[cfg(feature = "uniffi")]
uniffi::custom_newtype!(ProfileId, i64);

/// Normalized string buffer.
///
/// This type is guaranteed to be a non-empty string with no trailing or leading
/// whitespace.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Deref, Serialize)]
#[cfg_attr(
    feature = "poem",
    derive(poem_openapi::NewType),
    oai(from_json = false, from_parameter = false, from_multipart = false)
)]
#[debug("{_0:?}")]
pub struct NormString(String);

/// Attempted to turn a string into a [`NormString`], but the string was empty.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Error)]
#[display("string empty")]
pub struct StringEmpty;

impl TryFrom<String> for NormString {
    type Error = StringEmpty;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(StringEmpty)
    }
}

impl FromStr for NormString {
    type Err = StringEmpty;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s).ok_or(StringEmpty)
    }
}

#[cfg(feature = "uniffi")]
uniffi::custom_type!(NormString, String, {
    lower: |s| s.0,
    try_lift: |s| NormString::try_from(s).map_err(Into::into),
});

#[doc(hidden)]
pub trait TermPart: Sized {
    type IntoTerm;

    fn try_into_non_empty_string(self) -> Option<NormString>;

    fn into_term_with_headword(self) -> Self::IntoTerm;

    fn into_term_with_reading(self) -> Self::IntoTerm;
}

#[cfg(feature = "uniffi")]
uniffi::setup_scaffolding!();
