//! [Yomitan] dictionary format, specialized for Japanese.
//!
//! [Yomitan]: https://github.com/yomidevs/yomitan/

#[cfg(feature = "render-html")]
mod html;
pub mod structured;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Glossary {
    /// How frequently this word appears, as a ranking relative to other terms
    /// in this dictionary.
    pub popularity: i64,
    /// Tags applied to the glossary content.
    pub tags: Vec<GlossaryTag>,
    /// Structured glossary content.
    pub content: Vec<structured::Content>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Frequency {
    pub rank: Option<u64>,
    pub display: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pitch {
    /// What [mora] position the [downstep] is located on.
    ///
    /// This maps to a typical dictionary's "pitch position" entry:
    /// - 0: *heiban* (no downstep)
    /// - 1: *atamadaka*
    /// - greater than 1: *nakadaka* or *odaka*
    ///
    /// See [Binary pitch](https://en.wikipedia.org/wiki/Japanese_pitch_accent#Binary_pitch).
    ///
    /// [mora]: https://en.wikipedia.org/wiki/Mora_(linguistics)
    /// [downstep]: https://en.wikipedia.org/wiki/Downstep
    pub position: u64,
    /// What [morae][mora] positions have a [nasal] sound.
    ///
    /// [mora]: https://en.wikipedia.org/wiki/Mora_(linguistics)
    /// [nasal]: https://en.wikipedia.org/wiki/Nasal_consonant
    pub nasal: Vec<u64>,
    /// What [morae][mora] positions have a [devoiced] sound.
    ///
    /// [mora]: https://en.wikipedia.org/wiki/Mora_(linguistics)
    /// [devoiced]: https://en.wikipedia.org/wiki/Devoicing
    pub devoice: Vec<u64>,
}

/// Categorises a [`Glossary`] entry for a given [`Term`].
///
/// [`Term`]: crate::Term
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
    // TODO: what?
    // In kanji banks, if `category` is [`GlossaryTag::INDEX`], this is used as
    // the name of a dictionary.
    pub description: String,
    /// Order of this tag relative to other tags in the same dictionary.
    ///
    /// A higher value means the tag will be displayed later.
    pub order: i64,
}
