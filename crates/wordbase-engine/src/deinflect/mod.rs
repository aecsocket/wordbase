// mod lindera;

use {
    crate::Engine,
    anyhow::{Context, Result},
    futures::{Stream, StreamExt, stream},
    lindera::{
        dictionary::{DictionaryKind, load_dictionary_from_kind},
        mode::Mode,
        segmenter::Segmenter,
        token::Token,
        tokenizer::Tokenizer,
    },
};

#[derive(derive_more::Debug)]
pub struct Deinflectors {
    #[debug(skip)]
    tokenizer: Tokenizer,
}

impl Deinflectors {
    pub fn new() -> Result<Self> {
        let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic)
            .context("failed to load dictionary")?;
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        let tokenizer = Tokenizer::new(segmenter);
        Ok(Self { tokenizer })
    }
}

impl Engine {
    pub fn deinflect(&self, text: &str) -> impl Stream<Item = String> {
        stream::empty()
            .chain(identity(&self.deinflectors, text))
            .chain(lindera(&self.deinflectors, text))
    }
}

fn identity(_deinflectors: &Deinflectors, text: &str) -> impl Stream<Item = String> {
    stream::once(async move { text.to_owned() })
}

fn lindera(deinflectors: &Deinflectors, text: &str) -> impl Stream<Item = String> {
    fn lemma_of<'a>(token: &'a mut Token) -> Option<&'a str> {
        token.get_detail(7)
    }

    let Ok(mut tokens) = deinflectors.tokenizer.tokenize(text) else {
        return stream::empty::<String>().left_stream();
    };
    println!(
        "{text:?} -> {:?}",
        tokens.iter_mut().map(|t| lemma_of(t)).collect::<Vec<_>>()
    );
    let Some(mut token) = tokens.into_iter().next() else {
        return stream::empty().left_stream();
    };

    let lemmas = lemma_of(&mut token)
        .map(|lemma| vec![lemma.to_owned()])
        .unwrap_or_default();
    stream::iter(lemmas).right_stream()
}
