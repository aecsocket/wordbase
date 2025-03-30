// mod lindera;

use {
    crate::Engine,
    anyhow::{Context, Result},
    foldhash::{HashSet, HashSetExt},
    lindera::{
        dictionary::{DictionaryKind, load_dictionary_from_kind},
        mode::Mode,
        segmenter::Segmenter,
        tokenizer::Tokenizer,
    },
};

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Deinflectors {
    // pub lindera: lindera::Deinflector,
}

impl Deinflectors {
    pub fn new() -> Result<Self> {
        Ok(Self {
            // lindera: lindera::Deinflector::new().context("failed to create lindera deinflector")?,
        })
    }
}

impl Engine {
    #[expect(clippy::unused_async)] // todo
    pub async fn deinflect(&self, text: &str) -> Result<Vec<String>> {
        let mut lemmas = Vec::new();

        let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic)?;
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let tokenizer = Tokenizer::new(segmenter);
        if let Some(token) = tokenizer.tokenize(text)?.first_mut() {
            let details = token.details();
            if let Some(lemma) = details.get(7) {
                lemmas.push((*lemma).into());
            }
        }

        Ok(lemmas)
    }
}
