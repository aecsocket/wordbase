//! [Yomitan] dictionary format.
//!
//! [Yomitan]: https://github.com/yomidevs/yomitan/

pub mod schema;
pub mod structured;

mod html;
pub use html::*;

#[cfg(feature = "parse-yomitan")]
mod parse;
#[cfg(feature = "parse-yomitan")]
pub use parse::*;

use serde::{Deserialize, Serialize};

/// Categorises a [glossary] for a given [term].
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
    /// A higher value means the tag will be displayed later.
    pub order: i64,
}

/// Category of a [`GlossaryTag`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[expect(missing_docs, reason = "these tags are not documented well in Yomitan")]
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
