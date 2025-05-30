mod latin;
mod lindera;

use {
    crate::{Engine, IndexSet},
    anyhow::{Context, Result},
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
}

impl Engine {
    #[must_use]
    pub fn deinflect<'a>(&'a self, sentence: &'a str, cursor: usize) -> IndexSet<Deinflection<'a>> {
        iter::empty()
            // TODO: disable deinflectors based on language
            .chain(self.deinflectors.identity.deinflect(sentence, cursor))
            .chain(self.deinflectors.lindera.deinflect(sentence, cursor))
            .chain(self.deinflectors.latin.deinflect(sentence, cursor))
            .inspect(|deinflect| {
                debug_assert!(
                    sentence.get(deinflect.span.clone()).is_some(),
                    "text = {sentence:?}, cursor = {cursor}, span = {:?}",
                    deinflect.span
                );
            })
            .collect::<IndexSet<_>>()
    }
}

/// Single deinflection produced by [`Engine::deinflect`], mapping to a lemma
/// that should be looked up using [`Engine::lookup_lemma`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Deinflection<'s> {
    /// Byte span in the input sentence which this deinflection maps to.
    ///
    /// For example, if you input "食べなかった", the lemma would be "食べる", but the
    /// span would cover the entire range of "食べなかった", not just "食べな".
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
    use wordbase_api::Span;

    use crate::{FfiResult, Wordbase};

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

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    pub fn assert_deinflects<'a>(
        deinflector: &impl Deinflector,
        sentence: &str,
        cursor: usize,
        expected: impl IntoIterator<Item = Deinflection<'a>>,
    ) {
        assert_eq!(
            deinflector
                .deinflect(sentence, cursor)
                .collect::<IndexSet<_>>(),
            expected.into_iter().collect::<IndexSet<_>>(),
        );
    }

    pub fn deinf(text: &str) -> Deinflection {
        Deinflection::new(0, text, text)
    }

    #[test]
    fn identity() {
        let deinflector = Identity;
        assert_deinflects(&deinflector, "hello", 0, [deinf("hello")]);
        assert_deinflects(&deinflector, "hello world", 0, [deinf("hello world")]);
        assert_deinflects(
            &deinflector,
            "hello world",
            "hello ".len(),
            [deinf("world")],
        );
    }
}
