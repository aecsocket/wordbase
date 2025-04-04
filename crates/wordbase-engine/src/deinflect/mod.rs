// mod lindera;

use {
    crate::Engine,
    anyhow::Result,
    futures::{Stream, future::BoxFuture, stream::BoxStream},
    lindera::{
        dictionary::{DictionaryKind, load_dictionary_from_kind},
        mode::Mode,
        segmenter::Segmenter,
        tokenizer::Tokenizer,
    },
};

pub trait Deinflector {
    fn deinflect(&self, query: &str) -> BoxStream<'_, Result<String>>;
}

impl Engine {
    pub fn deinflect(&self, text: impl AsRef<str>) -> impl Stream<Item = String> {
        futures::stream::empty()
    }

    // #[expect(clippy::unused_async)] // todo
    // pub async fn deinflect(&self, text: &str) -> Result<Vec<String>> {
    //     let mut lemmas = Vec::new();

    //     let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic)?;
    //     let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    //     let tokenizer = Tokenizer::new(segmenter);
    //     if let Some(token) = tokenizer.tokenize(text)?.first_mut() {
    //         let details = token.details();
    //         if let Some(lemma) = details.get(7) {
    //             lemmas.push((*lemma).into());
    //         }
    //     }

    //     Ok(lemmas)
    // }
}
