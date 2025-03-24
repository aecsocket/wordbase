#![doc = include_str!("../README.md")]

// required for macro invocations
extern crate self as wordbase;

pub mod format;
pub mod glossary;
pub mod hook;
pub mod lang;
pub mod protocol;
pub mod record;
// pub mod render;
pub(crate) mod util;

use {
    derive_more::From,
    serde::{Deserialize, Serialize},
};

/// Invokes your own macro, passing in all existing [record] kinds as arguments.
///
/// [`Record`] and [`RecordKind`] are marked as `#[non_exhaustive]`, so adding
/// new variants is not a breaking change. However, this also means that if a
/// new variant is added, user code may not handle this case since it won't
/// cause a compilation failure. For many cases this is expected behavior, as
/// a user may not be able to handle or may not want to the new variant - for
/// example, a client which only cares about records for Japanese words (e.g.
/// pitch accent) should not have to care about a new record kind which only
/// applies to a European language dictionary.
///
/// However, some code may treat all record kinds equally (e.g. deserialization)
/// in which case adding a new variant would silently break existing code. To
/// avoid this, this macro can be used to ensure that code is generated for all
/// record kinds without you having to manually update it.
///
/// This macro also serves as the source of truth for all record kinds. If you
/// want to add a new record kind, add it here.
///
/// # Usage
///
/// Define a macro in a local scope which accepts the following pattern:
///
/// ```
/// macro_rules! my_macro {
///     ($($kind:ident($data_ty:path)),* $(,)?) => {
///         // your code here
///     };
/// }
/// ```
///
/// Then invoke it using:
///
/// ```
/// # macro_rules! my_macro {
/// #   ( $($kind:ident($data_ty:path)),* $(,)? ) => {
/// #       // your code here
/// #   }
/// # }
/// wordbase::for_record_kinds!(my_macro);
/// ```
///
/// - Use `$kind` to refer to the ident of the [`RecordKind`], i.e.
///   `GlossaryPlainText`
/// - Use `$data_ty` to refer to the data type that the [`Record`] variant
///   carries, i.e. `record::GlossaryPlainText`
///   - Note that this does not include the `wordbase::` prefix
///
/// # Examples
///
/// Generating top-level items:
///
/// ```
/// macro_rules! define_record_wrapper {
///     // note the single `{` at the end of this line...
///     ( $($kind:ident($data_ty:path)),* $(,)? ) => {
///         pub enum MyRecordWrapper {
///             $($kind {
///                 data: wordbase::$data_ty,
///                 extra: i32,
///             },)*
///         }
///     } // ...and the single `}` here
/// }
/// ```
///
/// Generating an expression:
///
/// ```
/// fn deserialize(kind: u16, data: &[u8]) {
///     macro_rules! deserialize_record {
///         // note the double `{` at the end of this line...
///         ( $($kind:ident($data_ty:path)),* $(,)? ) => {{
///             mod discrim {
///                 use wordbase::RecordKind;
///
///                 $(pub const $kind: u16 = RecordKind::$kind as u16;)*
///             }
///
///             match kind {
///                 $(discrim::$kind => from_json(data),)*
///                 _ => panic!("unknown kind"),
///             }
///         }} // ...and the double `}` here
///     }
///
///     let record = wordbase::for_record_kinds!(deserialize_record);
/// }
/// # fn from_json(data: &[u8]) { unimplemented!() }
/// ```
///
/// [record]: Record
#[macro_export]
macro_rules! for_record_kinds {
    ($macro:ident) => {
        $macro!(
            GlossaryPlainText(glossary::PlainText),
            GlossaryHtml(glossary::Html),
            Frequency(record::Frequency),
            JpnPitch(lang::jpn::Pitch),
            YomitanRecord(format::yomitan::Record),
        );
    };
}

/// Metadata for a collection of [records] in a Wordbase server.
///
/// This type only stores the metadata of a dictionary, such as the name and
/// version. [`Record`] stores a single entry for a [term] in a dictionary,
/// which is what you get when performing a lookup.
///
/// This does not necessarily represent an imported dictionary - for that, see
/// [`DictionaryState`].
///
/// [records]: Record
/// [term]: Term
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictionaryMeta {
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
}

/// State of an imported dictionary in a Wordbase server.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictionaryState {
    /// Unique identifier for this dictionary in the database.
    pub id: DictionaryId,
    /// What position [records] from this dictionary will be returned relative
    /// to other dictionaries.
    ///
    /// A higher position means [records] from this dictionary will be returned
    /// later, and should be displayed to the user with less priority.
    ///
    /// [records]: Record
    pub position: i64,
    /// Metadata.
    pub meta: DictionaryMeta,
}

/// Opaque and unique identifier for a single [`Dictionary`] in a database.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DictionaryId(pub i64);

/// Key for a [record] in a [dictionary], representing a single interpretation
/// of some text.
///
/// A term contains at least one of a headword or a reading:
/// - the headword is the [canonical form] of the term, as seen in a dictionary
/// - the reading is how the term is represented in an alternate form, e.g.
///   hiragana reading in Japanese.
///
/// # Examples
///
/// ```
/// # use wordbase::Term;
/// // English word "rust"
/// assert_eq!(Term::new("rust"), Term::Headword("rust".into()),);
///
/// // Greek word "σκουριά"
/// assert_eq!(Term::new("σκουριά"), Term::Headword("σκουριά".into()),);
///
/// // Japanese word "錆" ("さび")
/// assert_eq!(
///     Term::with_reading("錆", "さび"),
///     Term::Full {
///         headword: "錆".into(),
///         reading: Some("さび".into())
///     }
/// );
///
/// // Japanese word with only a reading
/// assert_eq!(Term::only_reading("さび"), Term::Reading("さび".into()),);
/// ```
///
/// [record]: Record
/// [dictionary]: Dictionary
/// [canonical form]: https://en.wikipedia.org/wiki/Lemma_(morphology)#Headword
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged, deny_unknown_fields)]
pub enum Term {
    /// Only a headword.
    Headword(String),
    /// Only a reading.
    Reading(String),
    /// Both a headword and reading.
    Full {
        /// Canonical form of the word.
        headword: String,
        /// Alternate form of the word.
        reading: String,
    },
}

impl Term {
    /// Creates a term with only a headword.
    #[must_use]
    pub fn new(headword: impl Into<String>) -> Self {
        Self::Headword(headword.into())
    }

    /// Creates a term with a headword and reading.
    #[must_use]
    pub fn with_reading(headword: impl Into<String>, reading: impl Into<String>) -> Self {
        Self::Full {
            headword: headword.into(),
            reading: reading.into(),
        }
    }

    /// Creates a term with only a reading.
    #[must_use]
    pub fn only_reading(reading: impl Into<String>) -> Self {
        Self::Reading(reading.into())
    }

    /// Creates a term from a headword and reading pair.
    ///
    /// If both are [`None`], returns [`None`].
    #[must_use]
    pub fn from_pair(headword: Option<String>, reading: Option<String>) -> Option<Self> {
        match (headword, reading) {
            (Some(headword), Some(reading)) => Some(Self::Full { headword, reading }),
            (Some(headword), None) => Some(Self::Headword(headword)),
            (None, Some(reading)) => Some(Self::Reading(reading)),
            (None, None) => None,
        }
    }

    /// Gets the headword if it is present.
    #[must_use]
    pub fn headword(&self) -> Option<&str> {
        match self {
            Self::Headword(headword) | Self::Full { headword, .. } => Some(headword),
            Self::Reading(_) => None,
        }
    }

    /// Gets the reading if it is present.
    #[must_use]
    pub fn reading(&self) -> Option<&str> {
        match self {
            Self::Headword(_) => None,
            Self::Reading(reading) | Self::Full { reading, .. } => Some(reading),
        }
    }
}

macro_rules! define_record_types { ($($kind:ident($data_ty:path)),* $(,)?) => {
/// Data for a single [term] in a [dictionary].
///
/// Dictionaries contain records for individual terms, and may contain
/// multiple records for the same term. Different dictionary formats store
/// different types of data for each term, so instead of attempting to normalize
/// all these types into a single universal type, we store all the data in its
/// original form (or converted to a slightly more structured form). These
/// different types of data are then expressed as different variants of this
/// record enum.
///
/// A record kind may also be specific to a single language, or a single
/// dictionary format. In this case, the variant name is prefixed with the
/// identifier of that language or format.
///
/// Since support for more dictionary formats may be added later, and adding a
/// new format must not break existing code, **all record-related data should be
/// treated as non-exhaustive** (and are indeed marked `#[non_exhaustive]`)
/// unless directly stated otherwise.
///
/// [term]: Term
/// [dictionary]: Dictionary
/// [content]: format::yomitan::structured::Content
#[derive(Debug, Clone, From, Serialize, Deserialize)]
#[expect(missing_docs, reason = "contained type of each variant provides docs")]
#[non_exhaustive]
pub enum Record { $($kind($data_ty),)* }

impl Record {
    /// Gets the [`RecordKind`] of this record.
    #[must_use]
    pub const fn kind(&self) -> RecordKind {
        match self {
            $(Self::$kind(_) => RecordKind::$kind,)*
        }
    }
}

/// Kind of a [record].
///
/// [record]: Record
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[expect(missing_docs, reason = "`Record` has more info")]
#[repr(u16)]
#[non_exhaustive]
pub enum RecordKind { $($kind,)* }

mod sealed {
    pub trait RecordType {}
}

/// Provides type bounds for variants of [`Record`], and guarantees that this
/// type can be converted into a variant of [`Record`].
///
/// This trait is sealed, and cannot be implemented by users.
pub trait RecordType:
    sealed::RecordType
    + Sized
    + Send
    + Sync
    + std::fmt::Debug
    + Clone
    + Serialize
    + serde::de::DeserializeOwned
    + Into<Record>
{
    /// [`RecordKind`] of this record data type.
    const KIND: RecordKind;
}

$(impl sealed::RecordType for $data_ty {}
impl RecordType for $data_ty {
    const KIND: RecordKind = RecordKind::$kind;
})*
}}

for_record_kinds!(define_record_types);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProfileId(pub i64);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileMeta {
    pub name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProfileState {
    pub id: ProfileId,
    pub meta: ProfileMeta,
    pub enabled_dictionaries: Vec<DictionaryId>,
}
