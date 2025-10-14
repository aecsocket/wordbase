//! See [`Deinflector`].

use {
    eyre::Result,
    std::{any::type_name, fmt::Debug},
    wordbase_types::Deinflection,
};

/// Reads text of arbitrary length and generates [`Deinflection`]s for use in
/// lookups.
///
/// Each generated [`Deinflection::lemma`] will be looked up using
/// [`LookupStorage`] to find appropriate records for the word, so it's OK if
/// the generated lemma is potentially not a real lemma, and therefore may not
/// have results in the storage. False positives are better than false negatives
/// in this case.
///
/// [`LookupStorage`]: crate::storage::LookupStorage
pub trait Deinflector: Send + Sync + Debug + 'static {
    /// Gets a unique, static, and persistent identifier for this deinflector.
    ///
    /// This is implemented as [`type_name::<Self>`][type_name] by default.
    fn id(&self) -> &'static str {
        type_name::<Self>()
    }

    /// Reads text of arbitrary length and generates [`Deinflection`]s.
    ///
    /// `cursor` is where the user's text cursor is currently located when
    /// looking up a word in this text. This can be used to generate more
    /// accurate lemmas, e.g. seeking backwards from the cursor to find the
    /// start of the word at the cursor.
    ///
    /// # Errors
    ///
    /// Errors if the lemmas could not be generated.
    ///
    /// [`LookupStorage`]: crate::storage::LookupStorage
    fn deinflect<'t>(&self, sentence: &'t str, cursor: usize) -> Result<Vec<Deinflection<'t>>>;
}
