mod latin;
mod lindera;

use {
    crate::{Engine, IndexSet},
    anyhow::{Context, Result},
    itertools::Itertools,
    serde::{Deserialize, Serialize},
    std::{borrow::Cow, iter, ops::Range},
};

pub trait Deinflector: Send + Sync + 'static {
    fn deinflect<'a>(
        &'a self,
        sentence: &'a str,
        cursor: usize,
    ) -> impl Iterator<Item = Deinflection<'a>>;
}

#[derive(Debug)]
pub struct Deinflectors {
    identity: Identity,
    lindera: lindera::Lindera,
    latin: latin::Latin,
}

impl Deinflectors {
    pub fn new() -> Result<Self> {
        Ok(Self {
            identity: Identity,
            lindera: lindera::Lindera::new().context("failed to create Lindera deinflector")?,
            latin: latin::Latin,
        })
    }

    fn deinflect<'a>(&'a self, sentence: &'a str, cursor: usize) -> IndexSet<Deinflection<'a>> {
        iter::empty()
            // TODO: disable deinflectors based on language
            .chain(self.identity.deinflect(sentence, cursor))
            .chain(self.lindera.deinflect(sentence, cursor))
            .chain(self.latin.deinflect(sentence, cursor))
            .inspect(|deinflect| {
                debug_assert!(
                    sentence.get(deinflect.span.clone()).is_some(),
                    "text = {sentence:?}, cursor = {cursor}, span = {:?}",
                    deinflect.span
                );
            })
            .sorted_by_key(|deinflect| deinflect.span.start)
            .collect::<IndexSet<_>>()
    }
}

impl Engine {
    #[must_use]
    pub fn deinflect<'a>(&'a self, sentence: &'a str, cursor: usize) -> IndexSet<Deinflection<'a>> {
        self.deinflectors.deinflect(sentence, cursor)
    }
}

/// Single deinflection produced by [`Engine::deinflect`], mapping to a lemma
/// that should be looked up using [`Engine::lookup_lemma`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Deinflection<'s> {
    /// Byte span in the input sentence which this deinflection maps to.
    ///
    /// For example, if you input "食べなかった", the lemma would be "食べる",
    /// but the span would cover the entire range of "食べなかった", not
    /// just "食べな".
    pub span: Range<usize>,
    /// Lemma to look up using the engine.
    ///
    /// This might not correspond to a slice of text in the input sentence! For
    /// example, "walked" would deinflect to "walk" (a slice of the input
    /// sentence), but "run" would deinflect to "ran", which is a newly
    /// allocated string entirely.
    pub lemma: Cow<'s, str>,
}

impl<'a> Deinflection<'a> {
    pub fn new(start: usize, src: &str, lemma: impl Into<Cow<'a, str>>) -> Self {
        Self {
            lemma: lemma.into(),
            span: start..(start + src.len()),
        }
    }
}

#[derive(Debug)]
struct Identity;

impl Deinflector for Identity {
    fn deinflect<'a>(
        &'a self,
        sentence: &'a str,
        cursor: usize,
    ) -> impl Iterator<Item = Deinflection<'a>> {
        sentence
            .get(cursor..)
            .map(|lemma| Deinflection::new(cursor, lemma, lemma))
            .into_iter()
    }
}

#[cfg(feature = "uniffi")]
const _: () = {
    use {
        crate::{FfiResult, Wordbase},
        wordbase_api::Span,
    };

    #[derive(uniffi::Record)]
    pub struct Deinflection {
        pub lemma: String,
        pub span: Span,
    }

    #[uniffi::export]
    impl Wordbase {
        pub fn deinflect(&self, sentence: &str, cursor: u64) -> FfiResult<Vec<Deinflection>> {
            let cursor = usize::try_from(cursor).context("cursor too large")?;
            Ok(self
                .0
                .deinflect(sentence, cursor)
                .into_iter()
                .map(|deinflect| {
                    anyhow::Ok(Deinflection {
                        lemma: deinflect.lemma.into_owned(),
                        span: deinflect.span.try_into().context("span too large")?,
                    })
                })
                .collect::<Result<Vec<Deinflection>, _>>()?)
        }
    }
};

#[allow(unused_macros, reason = "used in tests")]
macro_rules! sentence {
    (/ $a:literal) => {{
        let text = $a;
        (text, 0usize)
    }};
    ($a:literal /) => {{
        let text = $a;
        (text, $a.len())
    }};
    ($a:literal / $b:literal) => {{
        let text = concat!($a, $b);
        (text, $a.len())
    }};
    (/ $a:literal / $b:literal) => {{
        let text = concat!($a, $b);
        (text, 0usize, $a.len())
    }};
    ($a:literal / $b:literal / $c:literal) => {{
        let text = concat!($a, $b, $c);
        (text, $a.len(), concat!($a, $b).len())
    }};
}

#[allow(unused_imports, reason = "used in tests")]
pub(crate) use sentence;

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    pub fn assert_deinflects<'a>(
        deinflector: &impl Deinflector,
        (sentence, cursor): (&str, usize),
        expected: impl IntoIterator<Item = Deinflection<'a>>,
    ) {
        assert_eq!(
            deinflector
                .deinflect(sentence, cursor)
                .collect::<IndexSet<_>>(),
            expected.into_iter().collect::<IndexSet<_>>(),
        );
    }

    pub fn deinf(text: &str) -> Deinflection<'_> {
        Deinflection::new(0, text, text)
    }

    #[test]
    fn identity() {
        let deinflector = Identity;
        assert_deinflects(&deinflector, sentence!(/ "hello"), [deinf("hello")]);
        assert_deinflects(
            &deinflector,
            sentence!(/ "hello world"),
            [deinf("hello world")],
        );

        let (text, start) = sentence!("hello " / "world");
        assert_deinflects(
            &deinflector,
            (text, start),
            [Deinflection::new(start, "world", "world")],
        );
    }

    #[test]
    fn sentence() {
        assert_eq!(sentence!(/ "hello"), ("hello", 0usize));
        assert_eq!(sentence!("hello" /), ("hello", "hello".len()));
        assert_eq!(
            sentence!("hello " / "world"),
            ("hello world", "hello ".len())
        );
        assert_eq!(
            sentence!(/ "hello " / "world"),
            ("hello world", 0usize, "hello ".len())
        );
        assert_eq!(
            sentence!("hello " / "and" / " goodbye"),
            ("hello and goodbye", "hello ".len(), "hello and".len()),
        );
    }

    #[test]
    fn deinflections_starting_earlier_first() {
        // when a user clicks in the middle of a word, some later deinflectors
        // may end up producing lemmas which start *earlier* than the earlier
        // deinflectors. for example, in "アルコール", if you click in between
        // "アル" and "コール", we will try deinflecting "コール" first, and
        // then "アルコール". in this case, the text highlighted to the user
        // will be e.g. "アルコール", but the first results will be for "コール".
        //
        // to avoid this, we sort the deinflections by whichever ones have spans
        // that start the earliest. this test ensures that.
        let deinflectors = Deinflectors::new().unwrap();
        let (text, cursor1, cursor2) = sentence!("手を" / "アル" / "コールで消毒");
        let mut lemmas = deinflectors.deinflect(text, cursor2).into_iter();
        assert_eq!(
            lemmas.next().unwrap(),
            Deinflection::new(cursor1, "アルコール", "アルコール")
        );
        assert!(lemmas.next().is_some());
    }
}
