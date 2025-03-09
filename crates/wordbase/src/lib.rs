#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc)]

// required for macro invocations
extern crate self as wordbase;

pub mod format;
pub mod hook;
pub mod lang;
pub mod protocol;
pub mod record;
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
///     ( $($kind:ident($data_ty:path))* ) => {
///         // your code here
///     }
/// }
/// ```
///
/// Then invoke it using:
///
/// ```
/// # macro_rules! my_macro {
/// #   ( $($kind:ident($data_ty:path))* ) => {
/// #       // your code here
/// #   }
/// # }
/// wordbase::for_record_kinds!(my_macro);
/// ```
///
/// - Use `$kind` to refer to the ident of the [`RecordKind`], i.e.
///   `GlossaryPlainText`
/// - Use `$data_ty` to refer to the data type that the [`Record`] variant
///   carries, i.e. `wordbase::record::GlossaryPlainText`
///
/// # Examples
///
/// Generating top-level items:
///
/// ```
/// macro_rules! define_record_wrapper {
///     // note the single `{` at the end of this line...
///     ( $($kind:ident($data_ty:path))* ) => {
///         pub enum MyRecordWrapper {
///             $($kind {
///                 data: $data_ty,
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
///         ( $($kind:ident($data_ty:path))* ) => {{
///             mod discrim {
///                 use wordbase::RecordKind;
///
///                 $(pub const $kind: u16 = RecordKind::$kind as u16;)*
///             }
///
///             match kind {
///                 $(discrim::$kind => {
///                     from_json(data)
///                 })*
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
            // note: no comma separator
            GlossaryPlainText(wordbase::record::GlossaryPlainText)
            GlossaryHtml(wordbase::record::GlossaryHtml)
            Frequency(wordbase::record::Frequency)
            JpPitch(wordbase::lang::jp::Pitch)
            YomitanGlossary(wordbase::format::yomitan::Glossary)
        );
    };
}

/// Metadata for a collection of [records] in a Wordbase server.
///
/// This type only stores the metadata of a dictionary, such as the name and
/// version. Related types:
/// - [`Dictionary`]: data for a dictionary which has been imported into a
///   server, storing stateful data such as whether the dictionary is enabled,
///   and its user-configurable sorting position.
/// - [`Record`]: single entry for a [term] in a dictionary.
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
}

/// Imported collection of [records] in a Wordbase server.
///
/// This stores the [`DictionaryMeta`] plus stateful data which may be
/// configured by the user after import.
///
/// [records]: Record
/// [terms]: Term
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DictionaryState {
    /// Dictionary metadata.
    pub meta: DictionaryMeta,
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
    /// Whether this dictionary is used for returning records in lookup
    /// operations.
    ///
    /// This may be used to temporarily hide a specific dictionary.
    pub enabled: bool,
}

/// Opaque and unique identifier for a single [`Dictionary`] in a database.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DictionaryId(pub i64);

/// Key for a [record] in a [dictionary], representing a single interpretation
/// of some text.
///
/// # Examples
///
/// ```
/// # use wordbase::Term;
/// // English word "rust"
/// assert_eq!(
///     Term::without_reading("rust"),
///     Term {
///         headword: "rust".into(),
///         reading: None
///     }
/// );
///
/// // Greek word "σκουριά"
/// assert_eq!(
///     Term::without_reading("σκουριά"),
///     Term {
///         headword: "σκουριά".into(),
///         reading: None
///     }
/// );
///
/// // Japanese word "錆" ("さび")
/// assert_eq!(
///     Term::with_reading("錆", "さび"),
///     Term {
///         headword: "錆".into(),
///         reading: Some("さび".into())
///     }
/// );
/// ```
///
/// [record]: Record
/// [dictionary]: Dictionary
/// [glossaries]: Glossary
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Term {
    /// [Canonical form][headword] of the term.
    ///
    /// [headword]: https://en.wikipedia.org/wiki/Lemma_(morphology)#Headword
    pub headword: String,
    /// How the term is represented in an alternate form, e.g. hiragana reading
    /// in Japanese.
    ///
    /// If this is [`None`], the reading is the same as the [headword].
    ///
    /// [headword]: Term::headword
    pub reading: Option<String>,
}

impl Term {
    /// Creates a term with a headword and reading.
    #[must_use]
    pub fn with_reading(headword: impl Into<String>, reading: impl Into<String>) -> Self {
        Self {
            headword: headword.into(),
            reading: Some(reading.into()),
        }
    }

    /// Creates a term with only a headword.
    #[must_use]
    pub fn without_reading(headword: impl Into<String>) -> Self {
        Self {
            headword: headword.into(),
            reading: None,
        }
    }
}

macro_rules! define_record_types { ($($kind:ident($data_ty:path))*) => {
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
/// # Glossaries
///
/// The record kind which you are probably most interested in is the *glossary*,
/// which defines what a term actually means in human-readable terms - the
/// natural meaning of "dictionary entry". However, the content is left
/// deliberately undefined, and it is up to the dictionary to fill out what it
/// wants for its glossaries. Some dictionaries are monolingual, and may provide
/// a definition in the dictionary's own language. Others are bilingual, and
/// provide a translated meaning in the reader's native language.
///
/// Glossaries are complicated - there are many different formats of glossaries
/// in the wild, and each has their own format which they store content in,
/// sometimes bespoke. The `pyglossary` project has a [list of supported
/// glossary formats][formats] which is a good starting place to explore what
/// formats exist. But due to this fragmentation, we cannot sanely define a
/// single format to use for all glossaries, as we cannot guarantee that you
/// can convert from one to another.
///
/// Instead of defining a single universal glossary format, we support
/// glossaries in multiple formats. It is up to you to use the format which is
/// most convenient for you if it is present, or fallback to a different format
/// (potentially to multiple different formats).
///
// TODO: is this a good idea?
/// # Dynamic records
///
/// Some records, such as certain kinds of glossary records, may be *dynamic*.
/// This means that their contents aren't actually stored in the server's
/// database, but are instead computed on-the-fly from the data that it *does*
/// have when you make your request. It may even provide or omit entire records
/// based on what record kinds you request.
///
/// For example, the server may store a [Yomitan structured content][content]
/// record internally for a given term. If you request a [`YomitanGlossary`],
/// the server will provide you with this, but will *not* provide a
/// [`GlossaryHtml`] - you can compute that yourself from the structured content
/// you're given, and render it in your own way. However, if you don't request a
/// [`YomitanGlossary`], the server falls back to generating HTML by itself and
/// sending you the result - it will assume that you don't know what a Yomitan
/// glossary is, but still wants to provide a result.
///
/// [term]: Term
/// [terms]: Term
/// [dictionary]: Dictionary
/// [formats]: https://github.com/ilius/pyglossary/#supported-formats
/// [content]: format::yomitan::structured::Content
/// [`YomitanGlossary`]: format::yomitan::Glossary
/// [`GlossaryHtml`]: record::GlossaryHtml
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
    + std::fmt::Debug
    + Clone
    + Serialize
    + serde::de::DeserializeOwned
    + Into<Record>
{
    /// [`RecordKind`] of this record data type.
    const KIND: RecordKind;
}

$(
impl sealed::RecordType for $data_ty {}
impl RecordType for $data_ty {
    const KIND: RecordKind = RecordKind::$kind;
}
)*
}}

for_record_kinds!(define_record_types);
