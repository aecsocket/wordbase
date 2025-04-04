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
    std::borrow::Cow,
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
    pub fn deinflect<'a>(&'a self, text: &'a str) -> impl Stream<Item = Deinflection<'a>> {
        stream::empty()
            .chain(identity(&self.deinflectors, text))
            .chain(lindera(&self.deinflectors, text))
    }
}

#[derive(Debug, Clone)]
pub struct Deinflection<'a> {
    pub lemma: Cow<'a, str>,
    pub scan_len: usize,
}

fn identity<'a>(
    _deinflectors: &'a Deinflectors,
    text: &'a str,
) -> impl Stream<Item = Deinflection<'a>> {
    stream::once(async move {
        Deinflection {
            lemma: Cow::Borrowed(text),
            scan_len: text.len(),
        }
    })
}

fn lindera<'a>(
    deinflectors: &'a Deinflectors,
    text: &'a str,
) -> impl Stream<Item = Deinflection<'a>> {
    let Ok(tokens) = deinflectors.tokenizer.tokenize(text) else {
        return stream::empty().left_stream();
    };
    let Some(mut token) = tokens.into_iter().next() else {
        return stream::empty().left_stream();
    };
    let Some(lemma) = token.get_detail(7) else {
        return stream::empty().left_stream();
    };
    let lemma = lemma.to_owned();

    stream::once(async move {
        Deinflection {
            lemma: Cow::Owned(lemma),
            scan_len: token.byte_end.saturating_sub(token.byte_start),
        }
    })
    .right_stream()
}
