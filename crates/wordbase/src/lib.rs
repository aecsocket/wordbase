#![doc = include_str!("../README.md")]
#![allow(missing_docs)]

pub mod dict;

use {
    derive_more::From,
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

$($(
impl sealed::RecordType for dict::$dict_path::$record_kind {}

impl RecordType for dict::$dict_path::$record_kind {
    const KIND: RecordKind = RecordKind::[< $dict_kind $record_kind >];
}
)*)*
}}}
for_kinds!(define_types);

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
    /// Whether this dictionary is enabled for record lookups under the current
    /// [`Profile`].
    pub enabled: bool,
}

/// Metadata for a [`Dictionary`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DictionaryMeta {
    /// What kind of dictionary this was imported from.
    pub kind: DictionaryKind,
    /// Human-readable display name.
    ///
    /// This value is **not guaranteed to be unique** across a single server,
    /// however the server may treat this as a stable identifier for a
    /// dictionary in its unimported form (i.e. the archive itself), and use
    /// this to detect if you attempt to import an already-imported dictionary.
    pub name: String,
    /// Arbitrary version string.
    ///
    /// This does not guarantee to conform to any format, e.g. semantic
    /// versioning.
    pub version: String,
    /// Describes the content of this dictionary.
    pub description: Option<String>,
    /// Homepage URL where users can learn more about this dictionary.
    pub url: Option<String>,
    pub attribution: Option<String>,
}

impl DictionaryMeta {
    #[must_use]
    pub fn new(kind: DictionaryKind, name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            version: version.into(),
            description: None,
            url: None,
            attribution: None,
        }
    }
}

/// Opaque and unique identifier for a single [`Dictionary`] in a database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DictionaryId(pub i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum TermKind {
    Headword,
    Reading,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Term {
    kind: TermKind,
    text: String,
}

impl Term {
    #[must_use]
    pub fn new(kind: TermKind, text: impl Into<String>) -> Option<Self> {
        let text = text.into();
        if text.trim().is_empty() {
            None
        } else {
            Some(Self { kind, text })
        }
    }

    #[must_use]
    pub fn headword(text: impl Into<String>) -> Option<Self> {
        Self::new(TermKind::Headword, text.into())
    }

    #[must_use]
    pub fn reading(text: impl Into<String>) -> Option<Self> {
        Self::new(TermKind::Reading, text.into())
    }

    #[must_use]
    pub const fn kind(&self) -> TermKind {
        self.kind
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    /// Unique identifier for this profile in the database.
    pub id: ProfileId,
    /// Metadata.
    pub meta: ProfileMeta,
    /// Set of dictionaries which are [enabled] under this profile.
    ///
    /// [enabled]: DictionaryState::enabled
    pub enabled_dictionaries: Vec<DictionaryId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileMeta {
    /// Name of the profile.
    ///
    /// User-defined profiles will always have a name. If the name is missing,
    /// then this is the default profile made on startup.
    pub name: Option<String>,
    /// RGB accent color of the profile.
    ///
    /// This is purely aesthetic, but you can use this to style output for
    /// different profiles, and allow users to quickly differentiate between
    /// their profiles by color.
    pub accent_color: [f32; 3],
}

/// Opaque and unique identifier for a single [`Profile`] in a database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileId(pub i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lookup {
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
    ///
    /// Clients can also use this to fetch a small amount of info when doing an
    /// initial lookup, then fetch more records (e.g. pronunciation audio) when
    /// the user selects a specific term.
    pub record_kinds: Vec<RecordKind>,
}

/// Single record returned by the server in response to a [`Lookup`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecordLookup {
    /// ID of the [`Record`].
    pub id: RecordId,
    /// ID of the [`Dictionary`] from which the record was retrieved.
    pub source: DictionaryId,
    /// [`Term`] that this record is for.
    pub term: Term,
    /// [`Record`] that was found.
    pub record: Record,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RecordId(pub i64);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TexthookerSentence {
    pub process_path: String,
    pub sentence: String,
}
