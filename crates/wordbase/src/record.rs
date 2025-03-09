//! Generic record types which may be present in any dictionary.

use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

/// Definition of a [term] in plain text format.
///
/// This is the simplest glossary format, and should be used as a fallback
/// if there is no other way to express your glossary content. Similarly,
/// clients should only use this as a fallback source for rendering.
///
/// [term]: crate::Term
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deref, DerefMut, Serialize, Deserialize)]
pub struct GlossaryPlainText(pub String);

/// Definition of a [term] in HTML format.
///
/// This is a well-supported format which is common in many dictionaries,
/// and can be easily rendered by many clients (as long as you have access
/// to a [`WebView`] widget, or are rendering inside a browser).
///
/// [term]: crate::Term
/// [`WebView`]: https://en.wikipedia.org/wiki/WebView
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deref, DerefMut, Serialize, Deserialize)]
pub struct GlossaryHtml(pub String);

/// How often a given [term] appears in a [dictionary]'s [corpus].
///
/// Each text corpus which a dictionary is based on will naturally have some
/// terms appear more frequently than others. This type represents some of this
/// frequency information.
///
/// [term]: crate::Term
/// [dictionary]: crate::Dictionary
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
    /// Creates a new value from just a rank.
    #[must_use]
    pub const fn new(rank: u64) -> Self {
        Self {
            rank,
            display: None,
        }
    }
}
