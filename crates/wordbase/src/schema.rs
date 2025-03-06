use serde::{Deserialize, Serialize};

use crate::yomitan::structured;

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
pub struct Dictionary {
    /// Opaque and unique identifier for this dictionary in the database.
    pub id: DictionaryId,
    /// Human-readable display name.
    pub name: String,
    /// Arbitrary version string.
    ///
    /// This does not guarantee to conform to any existing format, e.g.
    /// semantic versioning.
    pub revision: String,
    /// Whether this dictionary is used for returning results in lookup
    /// operations.
    pub enabled: bool,
}

/// Opaque identifier for a single [`Dictionary`] in a database.
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
///     Term { expression: "rust".into(), reading: None }
/// );
///
/// // Greek word "σκουριά"
/// assert_eq!(
///     Term::without_reading("σκουριά"),
///     Term { expression: "σκουριά".into(), reading: None }
/// );
///
/// // Japanese word "錆" ("さび")
/// assert_eq!(
///     Term::with_reading("錆", "さび"),
///     Term { expression: "錆".into(), reading: Some("さび".into()) }
/// );
/// ```
///
/// [dictionary]: Dictionary
/// [glossaries]: Glossary
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Term {
    /// Canonical form of the term.
    pub expression: String,
    /// How the term is represented in its alternate form, e.g. kana.
    ///
    /// If this is [`None`], the reading is the same as the [expression].
    ///
    /// [expression]: Term::expression
    pub reading: Option<String>,
}

impl Term {
    /// Creates a term with an expression and reading.
    #[must_use]
    pub fn with_reading(expression: impl Into<String>, reading: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
            reading: Some(reading.into()),
        }
    }

    /// Creates a term with only an expression.
    #[must_use]
    pub fn without_reading(expression: impl Into<String>) -> Self {
        Self {
            expression: expression.into(),
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
/// This is the main content that you want to show a user for a term lookup. The
/// content is expressed as [structured content], which you are responsible for
/// rendering out into a format which you can display.
///
/// [term]: Term
/// [structured content]: structured
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Glossary {
    /// Tags for this glossary.
    pub tags: Vec<TermTag>,
    /// Content of this glossary.
    pub content: Vec<structured::Content>,
}

/// Categorises a [glossary] for a given [term].
///
/// These serve no functional purpose, but are useful for labelling and
/// categorising this entry.
///
/// [glossary]: Glossary
/// [term]: Term
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermTag {
    /// Human-readable name for this tag.
    pub name: String,
    /// Human-readable description of what this tag means for this term.
    pub description: String,
    /// What category this tag is defined as.
    pub category: Option<TagCategory>,
    /// Order of this tag relative to other tags in the same dictionary.
    ///
    /// A higher value means the tag will appear later.
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
    #[must_use]
    pub const fn new(rank: u64) -> Self {
        Self {
            rank,
            display_rank: None,
        }
    }

    #[must_use]
    pub fn with_display(rank: u64, display: impl Into<String>) -> Self {
        Self {
            rank,
            display_rank: Some(display.into()),
        }
    }
}

/// Japanese pitch accent information for a specific [term].
///
/// Japanese [dictionaries] may collect information on how a specific [term] is
/// [pronounced orally].
///
/// This type is currently specialized for Japanese pitch accent, however may be
/// replaced in the future to represent more general pitch accent.
///
/// [term]: Term
/// [dictionaries]: Dictionary
/// [pronounced orally]: https://en.wikipedia.org/wiki/Japanese_pitch_accent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pitch {
    pub position: u64,
    pub nasal: Vec<u64>,
    pub devoice: Vec<u64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LookupInfo {
    pub lemma: String,
    pub glossaries: Vec<(DictionaryId, Term, Glossary)>,
    pub frequencies: Vec<(DictionaryId, Term, Frequency)>,
    pub pitches: Vec<(DictionaryId, Term, Pitch)>,
}
