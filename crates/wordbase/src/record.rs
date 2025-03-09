//! Generic record types which may be present in any dictionary.

use serde::{Deserialize, Serialize};

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
