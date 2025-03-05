use serde::{Deserialize, Serialize};

/// Metadata for a dictionary.
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
    pub title: String,
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
/// [frequency data].
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
/// [frequency data]: Frequency
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Glossary {
    pub text: String,
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
    pub terms: Vec<(DictionaryId, Term)>,
    pub glossaries: Vec<(DictionaryId, Term, Glossary)>,
    pub frequencies: Vec<(DictionaryId, Term, Frequency)>,
    pub pitches: Vec<(DictionaryId, Term, Pitch)>,
}
