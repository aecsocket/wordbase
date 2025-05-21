//! [Yomitan] dictionary format, specialized for Japanese.
//!
//! See the [schemas list].
//!
//! [Yomitan]: https://github.com/yomidevs/yomitan/
//! [schemas list]: https://github.com/yomidevs/yomitan/blob/master/docs/making-yomitan-dictionaries.md#read-the-schemas

#[cfg(feature = "render-html")]
mod html;
pub mod structured;

use {
    super::jpn::PitchPosition,
    crate::FrequencyValue,
    serde::{Deserialize, Serialize},
};

/// What this term means, written in the dictionary's source language.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[serde(deny_unknown_fields)]
pub struct Glossary {
    /// How frequently this word appears, as a ranking relative to other terms
    /// in this dictionary.
    ///
    /// The engine will also use this value for sorting by frequency, so you do
    /// **not** have to sort by this value on the client side.
    pub popularity: i64,
    /// Tags applied to the glossary content.
    pub tags: Vec<GlossaryTag>,
    /// Structured glossary content.
    pub content: Vec<structured::Content>,
}

/// How often this term appears in this dictionary's corpus.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
#[serde(deny_unknown_fields)]
pub struct Frequency {
    /// Raw integer ranking value.
    ///
    /// The variant of [`FrequencyValue`] is determined by the [frequency mode]
    /// of the dictionary.
    ///
    /// [frequency mode]: https://github.com/yomidevs/yomitan/blob/d2fd7ec796bf3329abd6b92f2398e734d5042423/ext/data/schemas/dictionary-index-schema.json#L82
    pub value: Option<FrequencyValue>,
    /// Human-readable display form of [`Frequency::value`].
    ///
    /// Prefer displaying this to users if one is present.
    pub display: Option<String>,
}

/// Japanese pitch accent information.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
    pub position: PitchPosition,
    /// What [morae][mora] positions have a [nasal] sound.
    ///
    /// [mora]: https://en.wikipedia.org/wiki/Mora_(linguistics)
    /// [nasal]: https://en.wikipedia.org/wiki/Nasal_consonant
    pub nasal: Vec<PitchPosition>,
    /// What [morae][mora] positions have a [devoiced] sound.
    ///
    /// [mora]: https://en.wikipedia.org/wiki/Mora_(linguistics)
    /// [devoiced]: https://en.wikipedia.org/wiki/Devoicing
    pub devoice: Vec<PitchPosition>,
}

/// Categorises a [`Glossary`] entry for a given [`Term`].
///
/// [`Term`]: crate::Term
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
    // TODO: what?
    // In kanji banks, if `category` is [`GlossaryTag::INDEX`], this is used as
    // the name of a dictionary.
    pub description: String,
    /// Order of this tag relative to other tags in the same dictionary.
    ///
    /// A higher value means the tag will be displayed later.
    pub order: i64,
}
