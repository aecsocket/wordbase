mod latin;
mod lindera;

use {
    crate::{Engine, IndexSet},
    anyhow::{Context, Result},
    serde::{Deserialize, Serialize},
    std::{
        borrow::Cow,
        hash::{Hash, Hasher},
        iter,
        ops::Range,
    },
};

pub trait Deinflector: Send + Sync + 'static {
    fn deinflect<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Deinflection<'a>>;
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
    pub fn deinflect<'a>(&'a self, text: &'a str) -> IndexSet<Deinflection<'a>> {
        iter::empty()
            .chain(self.deinflectors.identity.deinflect(text))
            .chain(self.deinflectors.lindera.deinflect(text))
            .chain(self.deinflectors.latin.deinflect(text))
            .inspect(|deinflect| {
                debug_assert!(
                    text.get(deinflect.span.clone()).is_some(),
                    "text = {text:?}, span = {:?}",
                    deinflect.span
                );
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deinflection<'s> {
    pub lemma: Cow<'s, str>,
    pub span: Range<usize>,
}

impl<'a> Deinflection<'a> {
    pub fn new(lemma: impl Into<Cow<'a, str>>) -> Self {
        let lemma = lemma.into();
        Self {
            span: 0..lemma.len(),
            lemma,
        }
    }

    pub fn with_start(start: usize, lemma: impl Into<Cow<'a, str>>) -> Self {
        let lemma = lemma.into();
        Self {
            span: start..(start + lemma.len()),
            lemma,
        }
    }
}

impl Hash for Deinflection<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.lemma.hash(state);
    }
}

impl PartialEq for Deinflection<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.lemma == other.lemma
    }
}

impl Eq for Deinflection<'_> {}

#[derive(Debug)]
struct Identity;

impl Deinflector for Identity {
    fn deinflect<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Deinflection<'a>> {
        iter::once(Deinflection {
            lemma: Cow::Borrowed(text),
            span: 0..text.len(),
        })
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
        pub fn deinflect(&self, text: &str) -> FfiResult<Vec<Deinflection>> {
            Ok(self
                .0
                .deinflect(text)
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
fn assert_deinflects<'a>(
    deinflector: &impl Deinflector,
    text: &str,
    expected: impl IntoIterator<Item = Deinflection<'a>>,
) {
    assert_eq!(
        deinflector.deinflect(text).collect::<IndexSet<_>>(),
        expected.into_iter().collect::<IndexSet<_>>(),
    );
}
