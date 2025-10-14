use std::{borrow::Cow, ops::Range};

/// Result of using a deinflector to get the canonical form of a word in a
/// sentence.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "utoipa", derive(utoipa::ToSchema))]
pub struct Deinflection<'t> {
    /// Canonical, deinflected form of a word.
    ///
    /// A [*lemma*] is the canonical form of a word - the one you would find in
    /// a dictionary, without any inflections like tense. For example, the lemma
    /// of "running", "ran", and "run" is "run".
    ///
    /// If you look up this lemma using the engine's dictionary storage, you
    /// should get some appropriate dictionary results for this word.
    ///
    /// [lemma]: https://en.wikipedia.org/wiki/Lemma_%28morphology%29
    pub lemma: Cow<'t, str>,
    /// Where in the input text this lemma appears.
    ///
    /// Not that this may not be equivalent to [`Deinflection::lemma`]'s length
    /// if the lemma is not a substring of the input text. For example, in the
    /// input sentence,
    ///
    /// ```text
    /// He is running to the store.
    /// ```
    ///
    /// the word "running" will generate a deinflection for the lemma "run", but
    /// the two strings are not the same length. So the lemma "run" maps to this
    /// segment of the input text:
    ///
    /// ```text
    /// He is running to the store.
    ///       ^^^^^^^
    /// ```
    #[cfg_attr(feature = "utoipa", schema(inline, value_type = UsizeRange))]
    pub source_span: Range<usize>,
}

#[cfg(feature = "utoipa")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
struct UsizeRange {
    /// Lower bound of the range (inclusive).
    start: usize,
    /// Upper bound of the range (exclusive).
    end: usize,
}

impl<'t> Deinflection<'t> {
    /// Creates a new deinflection from a string in the source text, and the
    /// lemma that it generated.
    ///
    /// - `start` is the index of the start of `src` in the source text.
    /// - `src` is the string which deinflects to `lemma`.
    /// - `lemma` is the deinflected form of `src`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use wordbase_types::Deinflection;
    /// // assuming you have a source text, and are focusing on a specific word...
    /// let text = "He is running to the store.";
    /// let start = text.find("running").unwrap();
    ///
    /// // ...we generate the deinflection
    /// let deinflection = Deinflection::new(start, "running", "run");
    /// assert_eq!(deinflection.lemma, "run");
    /// assert_eq!(
    ///     deinflection.source_span,
    ///     ("He is ").len()..("He is running".len()),
    /// );
    /// ```
    pub fn new(start: usize, src: &str, lemma: impl Into<Cow<'t, str>>) -> Self {
        let lemma = lemma.into();
        Deinflection {
            source_span: start..(start + src.len()),
            lemma,
        }
    }

    /// Extracts this deinflection into a static, fully owned version.
    ///
    /// Clones the lemma if not already owned.
    #[must_use]
    pub fn into_owned(self) -> Deinflection<'static> {
        Deinflection {
            lemma: Cow::Owned(self.lemma.into_owned()),
            source_span: self.source_span,
        }
    }
}
