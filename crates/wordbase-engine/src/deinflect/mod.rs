mod lindera;

use {
    crate::Engine,
    ::lindera::{
        dictionary::{DictionaryKind, load_dictionary_from_kind},
        mode::Mode,
        segmenter::Segmenter,
        tokenizer::Tokenizer,
    },
    anyhow::{Context, Result},
    futures::{Stream, StreamExt, stream},
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
    let Ok(mut tokens) = deinflectors.tokenizer.tokenize(text) else {
        return stream::empty().left_stream();
    };

    // in text like "東京大学", lindera tokenizes it as "東京" and "大学"
    // our dictionary will have an entry for "東京", but we also want to check
    // if there's an entry for "東京大学"
    // to do this, we turn the first `TOKEN_LOOKAHEAD` tokens into a lemma,
    // then turn the first `TOKEN_LOOKAHEAD - 1` into another lemma, etc.
    let lemmas = (1..=TOKEN_LOOKAHEAD)
        .rev()
        .filter_map(move |up_to| {
            #[expect(clippy::option_if_let_else, reason = "borrow checker")]
            let (lookahead, rem) = if let Some(split) = tokens.split_at_mut_checked(up_to) {
                split
            } else {
                (tokens.as_mut_slice(), [].as_mut_slice())
            };

            // each slice of tokens actually turns into 2 lemmas:
            // - the result of joining the conjugation form of each token together
            // - the result of joining the lemma of each token together
            //
            // in UniDic, for a word like "食べる":
            //   conj form = 食べる (good)
            //       lemma = たべる (bad)
            // for a word like "東京":
            //   conj form = トウキョウ (bad)
            //       lemma = 東京 (good)
            //
            // so we need to do a lookup for both
            let conj_form = lookahead
                .iter_mut()
                .map(|token| token.get_detail(DETAIL_CONJUGATION_FORM))
                .collect::<Option<Vec<_>>>()?
                .join("");
            let full_lemma = lookahead
                .iter_mut()
                .map(|token| token.get_detail(DETAIL_LEMMA))
                .collect::<Option<Vec<_>>>()?
                .join("");

            let last_token = lookahead.last_mut()?;
            let last_end = last_token.byte_end;
            let last_pos = last_token.get_detail(DETAIL_PART_OF_SPEECH)?;

            // now we try to find where the last token ends
            // we go through all tokens after the last one, and find the last one
            // where the part of speech is no longer a "continuation" (e.g. an auxiliary
            // verb), then we use that last continuation token's end position as
            // the end of the word. this is a naive approach, but I don't know
            // how to do it better.
            let scan_len = rem
                .iter_mut()
                .filter_map(|next| {
                    let byte_end = next.byte_end;
                    next.get_detail(DETAIL_PART_OF_SPEECH)
                        .map(|pos| (pos, byte_end))
                })
                .take_while(|(next_pos, _)| is_continuation(last_pos, next_pos))
                .map(|(_, byte_end)| byte_end)
                .last()
                .unwrap_or(last_end);

            Some([
                Deinflection {
                    lemma: Cow::Owned(conj_form),
                    scan_len,
                },
                Deinflection {
                    lemma: Cow::Owned(full_lemma),
                    scan_len,
                },
            ])
        })
        .flatten();

    stream::iter(lemmas).right_stream()
}

// TODO: 終止形-一般 marks the end of a word
fn is_continuation(last_lookahead_pos: &str, next_pos: &str) -> bool {
    match last_lookahead_pos {
        // verb
        "動詞" => {
            matches!(next_pos, "助動詞") // auxiliary verb
        }
        // adjective
        "形容詞" => {
            matches!(next_pos, "接尾辞") // suffix
        }
        _ => false,
    }
}

const DETAIL_PART_OF_SPEECH: usize = 0;
const DETAIL_CONJUGATION_FORM: usize = 7;
const DETAIL_LEMMA: usize = 8;

const TOKEN_LOOKAHEAD: usize = 8;
