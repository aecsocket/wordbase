mod lindera;

use {
    crate::{Engine, IndexSet},
    anyhow::{Context, Result},
    serde::{Deserialize, Serialize},
    std::{
        borrow::Cow,
        hash::{Hash, Hasher},
        iter,
    },
};

pub trait Deinflector: Sized + Send + Sync + 'static {
    fn new() -> Result<Self>;

    fn deinflect<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Deinflection<'a>>;
}

#[derive(Debug)]
pub struct Deinflectors {
    identity: Identity,
    lindera: lindera::Lindera,
}

impl Deinflectors {
    pub fn new() -> Result<Self> {
        Ok(Self {
            identity: Identity,
            lindera: lindera::Lindera::new().context("failed to create Lindera deinflector")?,
        })
    }
}

impl Engine {
    #[must_use]
    pub fn deinflect<'a>(&'a self, text: &'a str) -> IndexSet<Deinflection<'a>> {
        iter::empty()
            .chain(self.deinflectors.identity.deinflect(text))
            .chain(self.deinflectors.lindera.deinflect(text))
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Deinflection<'s> {
    pub lemma: Cow<'s, str>,
    pub scan_len: usize,
}

impl<'a> Deinflection<'a> {
    pub fn new(lemma: impl Into<Cow<'a, str>>) -> Self {
        let lemma = lemma.into();
        Self {
            scan_len: lemma.len(),
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
    fn new() -> Result<Self> {
        Ok(Self)
    }

    fn deinflect<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Deinflection<'a>> {
        iter::once(Deinflection {
            lemma: Cow::Borrowed(text),
            scan_len: text.len(),
        })
    }
}
