use serde::{Deserialize, Serialize};

/// Categorises a [glossary] for a given [term].
///
/// These serve no functional purpose, but are useful for labelling and
/// categorising this entry.
///
/// [glossary]: crate::Glossary
/// [term]: crate::Term
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlossaryTag {
    /// Human-readable name for this tag.
    pub name: String,
    /// Human-readable description of what this tag means for this term.
    pub description: String,
    /// What category this tag is defined as.
    pub category: Option<TagCategory>,
    /// Order of this tag relative to other tags in the same dictionary.
    ///
    /// This is purely a rendering hint. A higher value means the tag will
    /// appear later.
    pub order: i64,
}

/// Category of a [`GlossaryTag`].
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
