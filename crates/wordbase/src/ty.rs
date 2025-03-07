use serde::{Deserialize, Serialize};

/// Metadata for a dictionary imported into a Wordbase server's database.
///
/// Dictionaries contain records for [terms][term] in their chosen language, which
/// provide info on:
/// - [`Glossary`]: the meaning(s) of a [term], either in the target language
///   (for a monolingual dictionary), or in a different language.
/// - [`Frequency`]: how often a [term] appears in the dictionary's [corpus],
///   and provides rankings for which [terms][term] are most common.
/// - [`Pitch`]: how a [term] may be pronounced orally.
///
/// [term]: Term
/// [corpus]: https://en.wikipedia.org/wiki/Text_corpus
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dictionary {
    /// Opaque and unique identifier for this dictionary in the database.
    pub id: DictionaryId,
    /// Human-readable display name.
    pub name: String,
    /// Arbitrary version string.
    ///
    /// This does not guarantee to conform to any existing format, e.g.
    /// semantic versioning.
    pub version: String,
    /// What position results from this dictionary will be displayed in,
    /// relative to other dictionaries.
    pub position: i64,
    /// Whether this dictionary is used for returning results in lookup
    /// operations.
    pub enabled: bool,
}

/// Opaque and unique identifier for a single [`Dictionary`] in a database.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DictionaryId(pub i64);

/// Key for a record in a [dictionary], representing a single interpretation of
/// some text.
///
/// This type is used as a key for other record information, such as
/// [glossaries].
///
/// # Examples
///
/// ```
/// # use wordbase::schema::Term;
/// // English word "rust"
/// assert_eq!(
///     Term::without_reading("rust"),
///     Term { headword: "rust".into(), reading: None }
/// );
///
/// // Greek word "σκουριά"
/// assert_eq!(
///     Term::without_reading("σκουριά"),
///     Term { headword: "σκουριά".into(), reading: None }
/// );
///
/// // Japanese word "錆" ("さび")
/// assert_eq!(
///     Term::with_reading("錆", "さび"),
///     Term { headword: "錆".into(), reading: Some("さび".into()) }
/// );
/// ```
///
/// [dictionary]: Dictionary
/// [glossaries]: Glossary
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

/// Provides the meaning or definition of a [term].
///
/// This defines what a [term] actually means in human-readable terms - the
/// natural meaning of "dictionary entry". However, the content is left
/// deliberately undefined, and it is up to the dictionary to fill out what it
/// wants for its glossaries. Some dictionaries are monolingual, and may provide
/// a definition in the dictionary's own language. Others are bilingual, and
/// provide a translated meaning in the reader's native language.
///
/// # Format
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
/// glossaries in multiple formats. When importing a dictionary, the importer
/// will convert it into one or multiple of these formats, and when looking up
/// a [term], data for each format may or may not be present. We leave it up to
/// you to determine which format is best for you to use. See the fields of this
/// struct to see which formats are supported.
///
/// If multiple formats are present on the same glossary, **they must represent
/// the same glossary content**, or at least as close as the two formats can
/// get. If one format is unable to express information that another can, then
/// the less detailed format should be emitted entirely.
///
/// Styling and other cosmetic details may be ignored, unless they directly
/// affect how the content is read and interpreted.
///
/// This type is marked as `#[non_exhaustive]` to allow adding new formats in
/// the future without breaking existing code.
///
/// # Examples
///
/// ```
/// # use wordbase::{TermTag, Glossary};
/// fn create_glossary(tags: Vec<TermTag>, html: String) -> Glossary {
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
/// [structured content]: structured
/// [formats]: https://github.com/ilius/pyglossary?tab=readme-ov-file#supported-formats
/// [HTML]: Glossary::html
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[non_exhaustive]
pub struct Glossary {
    /// Tags for this glossary.
    pub tags: Vec<TermTag>,
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
    /// to a [`WebView`] widget).
    ///
    /// [`WebView`]: https://en.wikipedia.org/wiki/WebView
    pub html: Option<String>,
}

/// Categorises a [glossary] for a given [term].
///
/// These serve no functional purpose, but are useful for labelling and
/// categorising this entry.
///
/// [glossary]: Glossary
/// [term]: Term
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TermTag {
    /// Human-readable name for this tag.
    pub name: String,
    /// Human-readable description of what this tag means for this term.
    pub description: String,
    /// What category this tag is defined as.
    pub category: Option<TagCategory>,
    /// Order of this tag relative to other tags in the same dictionary.
    ///
    /// This is purely a rendering hint for consumers. A higher value means
    /// the tag will appear later.
    pub order: i64,
}

/// Categorisation of a [term tag].
///
/// [term tag]: TermTag
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TagCategory {
    Name,
    Expression,
    Popular,
    Frequent,
    Archaism,
    Dictionary,
    Frequency,
    PartOfSpeech,
    Search,
    PronunciationDictionary,
}

/// How often a given [term] appears in a language.
///
/// [Dictionaries] may collect information on how often a [term] appears in its
/// corpus, and rank each [term] by how often they appear.
///
/// [Dictionaries]: Dictionary
/// [term]: Term
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frequency {
    /// How often this [term] appears in the [dictionary]'s corpus - a lower
    /// value means it appears more frequently.
    ///
    /// [term]: Term
    /// [dictionary]: Dictionary
    pub rank: u64,
    /// Human-readable display value for [`Frequency::rank`].
    ///
    /// If this is omitted, [`Frequency::rank`] should be presented directly.
    pub display_rank: Option<String>,
}

impl Frequency {
    /// Creates a value with a rank and display value.
    #[must_use]
    pub fn with_display(rank: u64, display: impl Into<String>) -> Self {
        Self {
            rank,
            display_rank: Some(display.into()),
        }
    }

    /// Creates a value from only a rank.
    #[must_use]
    pub const fn new(rank: u64) -> Self {
        Self {
            rank,
            display_rank: None,
        }
    }
}
