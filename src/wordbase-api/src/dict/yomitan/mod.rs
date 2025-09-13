//! [Yomitan] dictionary format, specialized for Japanese.
//!
//! See the [schemas list].
//!
//! [Yomitan]: https://github.com/yomidevs/yomitan/
//! [schemas list]: https://github.com/yomidevs/yomitan/blob/master/docs/making-yomitan-dictionaries.md#read-the-schemas

#[cfg(feature = "render-html")]
mod html;
#[cfg(feature = "render-html")]
pub use html::render_html;
use std::collections::HashMap;

pub mod structured;

use {super::jpn::PitchPosition, crate::FrequencyValue};

/// What this term means, written in the dictionary's source language.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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

/// Phonetic information for a term.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Phonetics {
    /// Phonetic transcriptions.
    pub transcriptions: Vec<PhoneticTranscription>,
}

/// One of the ways a term may be pronounced.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct PhoneticTranscription {
    /// [International Phonetic Alphabet][ipa] representation of the term.
    ///
    /// [ipa]: https://en.wikipedia.org/wiki/International_Phonetic_Alphabet
    pub ipa: String,
    /// Dictionary-specific tags for this transcription.
    pub tags: Vec<String>,
}

/// Information on a kanji character.
///
/// Terms associated with this record will always be [`Term::Headword`]s.
///
/// [`Term::Headword`]: crate::Term::Headword
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Kanji {
    /// [On'yomi] readings of this kanji.
    ///
    /// [On'yomi]: https://en.wikipedia.org/wiki/On%27yomi
    pub onyomi: Vec<String>,
    /// [Kun'yomi] readings of this kanji.
    ///
    /// [Kun'yomi]: https://en.wikipedia.org/wiki/Kun%27yomi
    pub kunyomi: Vec<String>,
    /// Meanings of this kanji.
    ///
    /// The language that meanings are written in is left undefined.
    pub meanings: Vec<String>,
    /// Extra dictionary-specific information about this kanji.
    ///
    /// Yomitan refers to this as "stats", but that's not quite accurate to the
    /// purpose of this field. It can also store extra data like the Unicode
    /// codepoint (key `Unicode`), composition (key `漢字構成`), or indexing
    /// radical (key `部首`).
    pub extra: HashMap<String, String>,
}

/// Categorises a [`Glossary`] entry for a given [`Term`].
///
/// [`Term`]: crate::Term
#[derive(Debug, Clone)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(deny_unknown_fields)
)]
#[cfg_attr(
    feature = "rkyv",
    derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize),
    rkyv(derive(Debug))
)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
