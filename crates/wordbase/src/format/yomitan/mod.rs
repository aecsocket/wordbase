//! [Yomitan] dictionary format, specialized for Japanese.
//!
//! [Yomitan]: https://github.com/yomidevs/yomitan/

mod html;
pub mod schema;
pub mod structured;

#[cfg(feature = "parse-yomitan")]
mod parse;
#[cfg(feature = "parse-yomitan")]
pub use parse::*;
use serde::{Deserialize, Serialize};

/// Yomitan-specific glossary format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glossary {
    /// Tags applied to this glossary record.
    pub tags: Vec<GlossaryTag>,
    /// Structured contents of this record.
    ///
    /// You can render this to HTML using [`maud::Render`].
    pub content: Vec<structured::Content>,
}

/// Categorises a glossary for a given [term].
///
/// [term]: crate::Term
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GlossaryTag {
    /// Human-readable name for this tag.
    pub name: String,
    /// What category this tag is defined as.
    ///
    /// This is an arbitrary (or empty) string, but Yomitan has several built-in
    /// tag categories [listed here][tags]. In addition, for kanji term records,
    /// certain tags have a special meaning.
    // TODO what special meanings?
    ///
    /// [tags]: https://github.com/yomidevs/yomitan/blob/09c55aeecd1d0912e3a664496a7a87640a41aa05/docs/making-yomitan-dictionaries.md#tag-categories
    pub category: String,
    /// Human-readable description of what this tag means for this term.
    ///
    /// In kanji banks, if `category` is [`GlossaryTag::INDEX`], this is used as
    /// the name of a dictionary.
    pub description: String,
    /// Order of this tag relative to other tags in the same dictionary.
    ///
    /// A higher value means the tag will be displayed later.
    pub order: i64,
}
