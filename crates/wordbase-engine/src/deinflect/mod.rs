// mod lindera;

use {
    crate::Engine,
    anyhow::{Context, Result},
    futures::{Stream, StreamExt, stream},
    lindera::{
        dictionary::{DictionaryKind, load_dictionary_from_kind},
        mode::Mode,
        segmenter::Segmenter,
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
    pub async fn deinflect<'a>(&'a self, text: &'a str) -> Vec<Deinflection<'a>> {
        let all = stream::empty()
            .chain(identity(&self.deinflectors, text))
            .chain(lindera(&self.deinflectors, text));

        let mut all = all.collect::<Vec<_>>().await;
        all.dedup_by(|a, b| a.lemma == b.lemma);
        all
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
    // _lindera_debug(deinflectors, text);
    let Ok(mut tokens) = deinflectors.tokenizer.tokenize(text) else {
        return stream::empty().left_stream();
    };

    let lemmas = (1..=TOKEN_LOOKAHEAD).rev().filter_map(move |up_to| {
        let tokens = tokens.get_mut(..up_to)?;
        let full_lemma = tokens
            .iter_mut()
            .map(|token| token.get_detail(DETAIL_LEMMA))
            .collect::<Option<Vec<_>>>()?
            .join("");

        Some(Deinflection {
            lemma: Cow::Owned(full_lemma),
            scan_len: tokens.last()?.byte_end,
        })
    });

    stream::iter(lemmas).right_stream()
}

fn _lindera_debug<'a>(deinflectors: &'a Deinflectors, text: &'a str) {
    let tokens = deinflectors.tokenizer.tokenize(text).unwrap();
    println!("TOKENS:");
    for mut token in tokens {
        println!("- {}", token.text);
        println!("  {}", token.details().join(", "));
    }
    println!("------");
}

fn pos_is_end_of_word(pos: &str) -> bool {
    pos == "名詞" || pos == "動詞" || pos == "形容詞"
}

const DETAIL_LEMMA: usize = 8;

const TOKEN_LOOKAHEAD: usize = 4;
