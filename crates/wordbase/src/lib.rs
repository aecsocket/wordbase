#![doc = include_str!("../README.md")]
#![allow(missing_docs)]

pub mod dict;

mod imp;
mod protocol;
pub use protocol::*;
use {
    derive_more::{Deref, Display, From},
    serde::{Deserialize, Serialize, de::DeserializeOwned},
    std::fmt::Debug,
};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u32)]
#[non_exhaustive]
pub enum DictionaryKind {
    $(
    #[doc = concat!("See [`dict::", stringify!($dict_path), "`].")]
    $dict_kind,
    )*
}

impl DictionaryKind {
    /// All variants of this enum.
    pub const ALL: &[Self] = &[$(Self::$dict_kind,)*];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem-openapi", derive(poem_openapi::Enum))]
#[repr(u32)]
#[non_exhaustive]
pub enum RecordKind {
    $($(
    #[doc = concat!("See [`dict::", stringify!($dict_path), "::", stringify!($record_kind), "`].")]
    [< $dict_kind $record_kind >],
    )*)*
}

impl RecordKind {
    /// All variants of this enum.
    pub const ALL: &[Self] = &[$($(Self::[< $dict_kind $record_kind >],)*)*];
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[non_exhaustive]
pub enum Record {
    $($(
    #[doc = concat!("See [`dict::", stringify!($dict_path), "::", stringify!($record_kind), "`].")]
    [< $dict_kind $record_kind >](dict::$dict_path::$record_kind),
    )*)*
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

pub trait RecordType:
    sealed::RecordType
    + Sized
    + Send
    + Sync
    + Debug
    + Clone
    + Serialize
    + DeserializeOwned
    + Into<Record>
{
    /// [`RecordKind`] variant of this record type.
    const KIND: RecordKind;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem-openapi", derive(poem_openapi::NewType))]
pub struct RecordId(pub i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem-openapi", derive(poem_openapi::NewType))]
pub struct DictionaryId(pub i64);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Term {
    Full {
        headword: NormString,
        reading: NormString,
    },
    Headword {
        headword: NormString,
    },
    Reading {
        reading: NormString,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem-openapi", derive(poem_openapi::Union))]
pub enum FrequencyValue {
    Rank(i64),
    Occurrence(i64),
}

impl FrequencyValue {
    #[must_use]
    pub const fn value(self) -> i64 {
        let (Self::Rank(n) | Self::Occurrence(n)) = self;
        n
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Unique identifier for this profile in the database.
    pub id: ProfileId,
    /// Metadata.
    pub meta: ProfileMeta,
    /// Set of [`Dictionary`] entries which are [enabled] under this profile.
    ///
    /// [enabled]: Dictionary::enabled
    pub enabled_dictionaries: Vec<DictionaryId>,
    /// Which [`Dictionary`] is used for sorting records by their frequencies.
    ///
    /// The user-set dictionary [position] always takes priority over any
    /// frequency sorting.
    ///
    /// [position]: Dictionary::position
    pub sorting_dictionary: Option<DictionaryId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMeta {
    /// Name of the profile.
    ///
    /// User-defined profiles will always have a name. If the name is missing,
    /// then this is the default profile made on startup.
    pub name: Option<NormString>,
    /// RGB accent color of the profile.
    ///
    /// This is purely aesthetic, but you can use this to style output for
    /// different profiles, and allow users to quickly differentiate between
    /// their profiles by color.
    pub accent_color: Option<[f32; 3]>,
}

/// Opaque and unique identifier for a [`Profile`] in the engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "poem-openapi", derive(poem_openapi::NewType))]
pub struct ProfileId(pub i64);

#[derive(Display, Clone, PartialEq, Eq, Hash, Deref, Serialize)]
pub struct NormString(String);

#[doc(hidden)]
pub trait TermPart: Sized {
    type IntoTerm;

    fn try_into_non_empty_string(self) -> Option<NormString>;

    fn into_term_with_headword(self) -> Self::IntoTerm;

    fn into_term_with_reading(self) -> Self::IntoTerm;
}
