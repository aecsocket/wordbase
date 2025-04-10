#![doc = include_str!("../README.md")]
#![allow(missing_docs)]

pub mod dict;

mod protocol;
use foldhash::HashMap;
pub use protocol::*;
use {
    derive_more::{Deref, Display, From},
    serde::{Deserialize, Serialize, de::DeserializeOwned},
    std::{
        fmt::{self, Debug},
        mem,
    },
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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
    pub attribution: Option<String>,
}

impl DictionaryMeta {
    #[must_use]
    pub fn new(kind: DictionaryKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            version: None,
            description: None,
            url: None,
            attribution: None,
        }
    }
}

/// Opaque and unique identifier for a single [`Dictionary`] in a database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

impl Term {
    #[must_use]
    pub fn new(headword: impl TermPart, reading: impl TermPart) -> Option<Self> {
        match (
            headword.try_into_non_empty_string(),
            reading.try_into_non_empty_string(),
        ) {
            (Some(headword), Some(reading)) => Some(Self::Full { headword, reading }),
            (Some(headword), None) => Some(Self::Headword { headword }),
            (None, Some(reading)) => Some(Self::Reading { reading }),
            (None, None) => None,
        }
    }

    #[must_use]
    pub fn from_headword<T: TermPart>(headword: T) -> T::IntoTerm {
        headword.into_term_with_headword()
    }

    #[must_use]
    pub fn from_reading<T: TermPart>(reading: T) -> T::IntoTerm {
        reading.into_term_with_reading()
    }

    #[must_use]
    pub fn headword(&self) -> Option<&NormString> {
        match self {
            Self::Full { headword, .. } | Self::Headword { headword } => Some(headword),
            Self::Reading { .. } => None,
        }
    }

    #[must_use]
    pub fn reading(&self) -> Option<&NormString> {
        match self {
            Self::Full { reading, .. } | Self::Reading { reading } => Some(reading),
            Self::Headword { .. } => None,
        }
    }

    #[must_use]
    pub fn headword_mut(&mut self) -> Option<&mut NormString> {
        match self {
            Self::Full { headword, .. } | Self::Headword { headword } => Some(headword),
            Self::Reading { .. } => None,
        }
    }

    #[must_use]
    pub fn reading_mut(&mut self) -> Option<&mut NormString> {
        match self {
            Self::Full { reading, .. } | Self::Reading { reading } => Some(reading),
            Self::Headword { .. } => None,
        }
    }

    pub fn set_headword(&mut self, new: NormString) -> Option<NormString> {
        match self {
            Self::Full { headword, .. } | Self::Headword { headword } => {
                Some(mem::replace(headword, new))
            }
            Self::Reading { reading } => {
                // CORRECTNESS: this non-empty string will never be accessed
                let reading = mem::replace(reading, NormString::new_unchecked(String::new()));
                *self = Self::Full {
                    headword: new,
                    reading,
                };
                None
            }
        }
    }

    pub fn set_reading(&mut self, new: NormString) -> Option<NormString> {
        match self {
            Self::Full { reading, .. } | Self::Reading { reading } => {
                Some(mem::replace(reading, new))
            }
            Self::Headword { headword } => {
                // CORRECTNESS: this non-empty string will never be accessed
                let headword = mem::replace(headword, NormString::new_unchecked(String::new()));
                *self = Self::Full {
                    headword,
                    reading: new,
                };
                None
            }
        }
    }

    #[must_use]
    pub fn take_headword(self) -> Option<NormString> {
        match self {
            Self::Full { headword, .. } | Self::Headword { headword } => Some(headword),
            Self::Reading { .. } => None,
        }
    }

    #[must_use]
    pub fn take_reading(self) -> Option<NormString> {
        match self {
            Self::Full { reading, .. } | Self::Reading { reading } => Some(reading),
            Self::Headword { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
    pub config: ProfileConfig,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    pub anki_deck: Option<NormString>,
    pub anki_model: Option<NormString>,
    #[serde(default)]
    pub anki_model_fields: HashMap<NormString, NormString>,
}

/// Opaque and unique identifier for a single [`Profile`] in a database.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileId(pub i64);

#[derive(Display, Clone, PartialEq, Eq, Hash, Deref, Serialize)]
pub struct NormString(String);

impl NormString {
    #[must_use]
    pub fn new(string: impl Into<String>) -> Option<Self> {
        let string: String = string.into();
        let trimmed = string.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed == string {
            Some(Self(string))
        } else {
            Some(Self(trimmed.to_string()))
        }
    }

    #[must_use]
    pub fn new_unchecked(string: String) -> Self {
        Self(string)
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Debug for NormString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for NormString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = NormString;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "non-empty string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                NormString::new(v).ok_or_else(|| E::custom("string must be non-empty"))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                NormString::new(v).ok_or_else(|| E::custom("string must be non-empty"))
            }
        }

        deserializer.deserialize_string(Visitor)
    }
}

#[doc(hidden)]
pub trait TermPart: Sized {
    type IntoTerm;

    fn try_into_non_empty_string(self) -> Option<NormString>;

    fn into_term_with_headword(self) -> Self::IntoTerm;

    fn into_term_with_reading(self) -> Self::IntoTerm;
}

impl TermPart for NormString {
    type IntoTerm = Term;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        Some(self)
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        Term::Headword { headword: self }
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        Term::Reading { reading: self }
    }
}

impl TermPart for Option<NormString> {
    type IntoTerm = Option<Term>;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        self
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        self.map(TermPart::into_term_with_headword)
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        self.map(TermPart::into_term_with_reading)
    }
}

impl TermPart for String {
    type IntoTerm = Option<Term>;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        NormString::new(self)
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        NormString::new(self).into_term_with_headword()
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        NormString::new(self).into_term_with_reading()
    }
}

impl TermPart for Option<String> {
    type IntoTerm = Option<Term>;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        self.and_then(TermPart::try_into_non_empty_string)
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        self.and_then(TermPart::into_term_with_headword)
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        self.and_then(TermPart::into_term_with_reading)
    }
}

impl TermPart for &str {
    type IntoTerm = Option<Term>;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        NormString::new(self)
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        NormString::new(self).map(|headword| Term::Headword { headword })
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        NormString::new(self).map(|reading| Term::Reading { reading })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term_api() {
        const TEST: &str = "test";

        let test = NormString::new(TEST).unwrap();

        assert_eq!(Term::from_headword(""), None);
        assert_eq!(
            Term::from_headword(TEST),
            Some(Term::Headword {
                headword: test.clone(),
            })
        );
        let mut term = Term::from_headword(test.clone());
        assert_eq!(
            term,
            Term::Headword {
                headword: test.clone()
            }
        );
        term.set_reading(test.clone());
        assert_eq!(
            term,
            Term::Full {
                headword: test.clone(),
                reading: test.clone()
            }
        );

        assert_eq!(Term::from_reading(""), None);
        assert_eq!(
            Term::from_reading(TEST),
            Some(Term::Reading {
                reading: test.clone()
            })
        );
        let mut term = Term::from_reading(test.clone());
        assert_eq!(
            term,
            Term::Reading {
                reading: test.clone(),
            }
        );
        term.set_headword(test.clone());
        assert_eq!(
            term,
            Term::Full {
                headword: test.clone(),
                reading: test
            }
        );
    }
}
