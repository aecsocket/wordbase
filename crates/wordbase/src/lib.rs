#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![warn(clippy::missing_errors_doc)]

pub mod format;
pub mod hook;
pub mod lang;
pub mod protocol;
pub(crate) mod util;

use {
    derive_more::From,
    serde::{Deserialize, Serialize},
};

/// Collection of [records] for [terms] imported into a Wordbase server.
///
/// This type stores the metadata of a dictionary, not the records themselves.
///
/// [records]: Record
/// [terms]: Term
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dictionary {
    /// Unique identifier for this dictionary in the database.
    pub id: DictionaryId,
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
/// # use wordbase::schema::Term;
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

macro_rules! record_kinds {
    ( $($kind:ident($data_ty:path)),* $(,)? ) => {
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
        /// [terms]: Term
        /// [dictionary]: Dictionary
        #[derive(Debug, Clone, From, Serialize, Deserialize)]
        #[expect(missing_docs, reason = "contained type of each variant provides docs")]
        #[non_exhaustive]
        pub enum Record {
            $($kind($data_ty),)*
        }

        /// Kind of a [record].
        ///
        /// [record]: Record
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[expect(missing_docs, reason = "`Record` has more info")]
        #[repr(u16)]
        #[non_exhaustive]
        pub enum RecordKind {
            $($kind,)*
        }
    };
}

record_kinds! {
    // generic
    Glossary(Glossary),
    Frequency(Frequency),
    // language-specific
    JpPitch(lang::jp::Pitch),
    // format-specific
}

/// Provides the meaning or definition of a [term].
///
/// This defines what a term actually means in human-readable terms - the
/// natural meaning of "dictionary entry". However, the content is left
/// deliberately undefined, and it is up to the [dictionary] to fill out what it
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
/// If multiple formats are present on the same glossary, **they must represent
/// the same content**, or at least as close as the two formats can get. If one
/// format is unable to express information that another can, then the less
/// detailed format should be omitted entirely.
///
/// Styling and other cosmetic details may be ignored, unless they directly
/// affect how the content is read and interpreted.
///
/// A glossary may also contain language-specific or dictionary format-specific
/// fields. In this case, the field name is prefixed with the identifier of that
/// language or format.
///
/// # Examples
///
/// ```
/// # use wordbase::Glossary;
/// fn create_glossary(html: String) -> Glossary {
///     // we can't create a value using a struct expression,
///     // since it's `#[non_exhaustive]`, so we make an empty one first...
///     let mut glossary = Glossary::default();
///     // then set our content
///     glossary.html = html;
///     glossary
/// }
///
/// fn display_glossary(glossary: &Glossary) {
///     // let's assume we're drawing some widgets in a UI toolkit,
///     // and we want to render this glossary in our UI
///     if let Some(content) = &glossary.html {
///         // we'll prioritise HTML, since we can load that directly in a WebView
///         load_into_web_view(content);
///     } else if let Some(content) = &glossary.plain_text {
///         // if the glossary has no HTML content, we fall back to plain text
///         // fortunately, a WebView can render text directly as well
///         load_into_web_view(content);
///     } else {
///         // we can't render this glossary!
///     }
/// }
/// # fn load_into_web_view(html: &str) { unreachable!() }
/// ```
///
/// [term]: Term
/// [dictionary]: Dictionary
/// [formats]: https://github.com/ilius/pyglossary/#supported-formats
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Glossary {
    // generic
    /// Plain text format.
    ///
    /// This is the simplest glossary format, and should be used as a fallback
    /// if there is no other way to express your glossary content. Similarly,
    /// consumers should only use this as a fallback source for rendering.
    pub plain_text: Option<String>,
    /// HTML content of this definition.
    ///
    /// This is a well-supported format which is common in many dictionaries,
    /// and can be easily rendered by many clients (as long as you have access
    /// to a [`WebView`] widget, or are rendering inside a browser).
    ///
    /// [`WebView`]: https://en.wikipedia.org/wiki/WebView
    pub html: Option<String>,

    // language-specific

    // format-specific
    /// ([`format::yomitan`]) Category tags for this glossary entry.
    pub yomitan_tags: Vec<format::yomitan::GlossaryTag>,
}

/// How often a given [term] appears in a [dictionary]'s [corpus].
///
/// Each text corpus which a dictionary is based on will naturally have some
/// terms appear more frequently than others. This type represents some of this
/// frequency information.
///
/// [term]: Term
/// [dictionary]: Dictionary
/// [corpus]: https://en.wikipedia.org/wiki/Text_corpus
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frequency {
    /// This [term]'s position in the frequency ranking of this [dictionary]'s
    /// [corpus].
    ///
    /// A lower ranking means this term appears more frequently.
    ///
    /// [term]: Term
    /// [dictionary]: Dictionary
    /// [corpus]: https://en.wikipedia.org/wiki/Text_corpus
    pub rank: u64,
    /// Human-readable display value for [`Frequency::rank`].
    ///
    /// If this is omitted, [`Frequency::rank`] should be presented directly.
    pub display: Option<String>,
}

impl Frequency {
    /// Creates a value from a rank.
    #[must_use]
    pub fn new(rank: u64) -> Self {
        Self {
            rank,
            ..Default::default()
        }
    }
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
