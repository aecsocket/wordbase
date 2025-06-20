// TODO: cases to handle:
// - ㌀ -> アパート
// - ２０日 -> 20日

use {
    super::{Deinflection, Deinflector},
    anyhow::{Context as _, Result},
    itertools::Itertools,
    lindera::{
        character_filter::{
            BoxCharacterFilter,
            unicode_normalize::{UnicodeNormalizeCharacterFilter, UnicodeNormalizeKind},
        },
        dictionary::{DictionaryKind, load_dictionary_from_kind},
        mode::Mode,
        segmenter::Segmenter,
        token::Token,
        tokenizer::Tokenizer,
    },
    std::{borrow::Cow, cmp},
};

#[derive(derive_more::Debug)]
pub struct Lindera {
    #[debug(skip)]
    tokenizer: Tokenizer,
    lookahead: usize,
}

const TOKEN_LOOKAHEAD: usize = 8;

impl Lindera {
    pub fn new() -> Result<Self> {
        Self::with_lookahead(TOKEN_LOOKAHEAD)
    }

    pub fn with_lookahead(lookahead: usize) -> Result<Self> {
        let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic)
            .context("failed to load dictionary")?;
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);

        let mut tokenizer = Tokenizer::new(segmenter);
        tokenizer.append_character_filter(BoxCharacterFilter::from(
            UnicodeNormalizeCharacterFilter::new(UnicodeNormalizeKind::NFKC),
        ));

        Ok(Self {
            tokenizer,
            lookahead,
        })
    }
}

impl Deinflector for Lindera {
    fn deinflect<'a>(
        &'a self,
        sentence: &'a str,
        cursor: usize,
    ) -> impl Iterator<Item = Deinflection<'a>> {
        let Some(text) = sentence.get(cursor..) else {
            return Vec::new().into_iter();
        };
        let Ok(mut tokens) = self.tokenizer.tokenize(text) else {
            return Vec::new().into_iter();
        };

        // some tokens may genuinely not be able to be mapped to `Details`,
        // i.e. an UNK token, so we filter them out, rather than failing entirely
        let mut tokens = tokens
            .iter_mut()
            .filter_map(Details::new)
            .collect::<Vec<_>>();

        // in text like "東京大学", lindera tokenizes it as "東京" and "大学"
        // our dictionary will have an entry for "東京", but we also want to check
        // if there's an entry for "東京大学"
        // to do this, we turn the first `lookahead` tokens into a lemma,
        // then turn the first `lookahead - 1` into another lemma, etc.
        let lemmas = (1..=self.lookahead)
            .rev()
            .filter_map(move |up_to| {
                #[expect(clippy::option_if_let_else, reason = "borrow checker")]
                let (lookahead, rem) = if let Some(split) = tokens.split_at_mut_checked(up_to) {
                    split
                } else {
                    // we may be trying to join the next L tokens into a lemma,
                    // but we only have N tokens, where N < L
                    (tokens.as_mut_slice(), [].as_mut_slice())
                };

                // each slice of tokens actually turns into 2 lemmas:
                // - the result of joining the lemma of each token together
                // - the result of joining the orthography of each token together
                //
                // in UniDic, for a word like "食べる":
                //     lemma = 食べる (good)
                //     ortho = たべる (bad)
                // for a word like "東京":
                //     lemma = トウキョウ (bad)
                //     ortho = 東京 (good)
                //
                // so we need to do a lookup for both
                let full_lemma = lookahead
                    .iter_mut()
                    .map(|token| token.lemma)
                    .collect::<Vec<_>>()
                    .join("");
                let full_ortho = lookahead
                    .iter_mut()
                    .map(|token| token.orthography)
                    .collect::<Vec<_>>()
                    .join("");

                let last_lookahead = lookahead.last_mut()?;
                let word_last_token = rem
                    .iter_mut()
                    .take_while_inclusive(|token| !is_word_ending(token))
                    .take_while(|token| is_word_continuation(last_lookahead, token))
                    .last();
                let scan_len =
                    word_last_token.map_or(last_lookahead.byte_end, |token| token.byte_end);
                let scan_end = cursor + scan_len;
                let span = cursor..scan_end;
                let scanned_text = sentence.get(span.clone()).unwrap_or_default();

                // The order in which we return `full_lemma` and `full_ortho` matters!
                // `full_lemma` will usually be the kanji form of a word,
                // even if the input text was in the kana form.
                // We want to first return the form which is closest to `scanned_text`.
                // i.e. if the input is in kana, we want to give the deinflection in kana.
                // For this we'll use Levenshtein distance.
                let lemma_distance = distance::levenshtein(scanned_text, &full_lemma);
                let ortho_distance = distance::levenshtein(scanned_text, &full_ortho);
                let deinflection_lemma = Deinflection {
                    lemma: Cow::Owned(full_lemma),
                    span: span.clone(),
                };
                let deinflection_ortho = Deinflection {
                    lemma: Cow::Owned(full_ortho),
                    span,
                };

                match lemma_distance.cmp(&ortho_distance) {
                    cmp::Ordering::Less | cmp::Ordering::Equal => {
                        Some([deinflection_lemma, deinflection_ortho])
                    }
                    cmp::Ordering::Greater => Some([deinflection_ortho, deinflection_lemma]),
                }
            })
            .flatten();
        lemmas.collect::<Vec<_>>().into_iter()
    }
}

// based on the `List of features` here: https://clrd.ninjal.ac.jp/unidic/faq.html
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Details<'a> {
    byte_start: usize,
    byte_end: usize,
    pos1: &'a str,
    pos2: &'a str,
    pos3: &'a str,
    pos4: &'a str,
    conjugation_type: &'a str,
    conjugation_form: &'a str,
    lexeme_form: &'a str,
    lemma: &'a str,
    orthography: &'a str,
    pronunciation: &'a str,
    orthography_base: &'a str,
    pronunciation_base: &'a str,
    origin: &'a str,
    word_subtype1: &'a str,
    word_subtype2: &'a str,
    word_subtype3: &'a str,
    alternate_form: &'a str,
}

impl<'a> Details<'a> {
    fn new(token: &'a mut Token) -> Option<Self> {
        let byte_start = token.byte_start;
        let byte_end = token.byte_end;
        let mut details = token.details().into_iter();
        Some(Self {
            byte_start,
            byte_end,
            pos1: details.next()?,
            pos2: details.next()?,
            pos3: details.next()?,
            pos4: details.next()?,
            conjugation_type: details.next()?,
            conjugation_form: details.next()?,
            lexeme_form: details.next()?,
            lemma: details.next()?,
            orthography: details.next()?,
            orthography_base: details.next()?,
            pronunciation: details.next()?,
            pronunciation_base: details.next()?,
            origin: details.next()?,
            word_subtype1: details.next()?,
            word_subtype2: details.next()?,
            word_subtype3: details.next()?,
            alternate_form: details.next()?,
        })
    }
}

fn is_word_ending(token: &Details) -> bool {
    matches!(
        token.conjugation_form,
        "終止形-一般" // terminal form
    )
}

fn is_word_continuation(last_lookahead: &Details, token: &Details) -> bool {
    match last_lookahead.pos1 {
        // verb
        "動詞" => {
            matches!(token.pos1, "助動詞") // auxiliary verb
        }
        // adjective
        "形容詞" => {
            matches!(
                token.pos1,
                "接尾辞" // suffix
                | "助動詞" // auxiliary verb
            )
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::deinflect::{
            sentence,
            tests::{assert_deinflects, deinf},
        },
        std::sync::LazyLock,
    };

    #[test]
    fn generate_details() {
        let taberu = Details {
            byte_start: 0,
            byte_end: 0,  // overridden later
            pos1: "動詞", // verb
            pos2: "一般", // general
            pos3: "*",
            pos4: "*",
            conjugation_type: "下一段-バ行", // ichidan
            conjugation_form: "終止形-一般", // terminal form
            lexeme_form: "タベル",
            lemma: "食べる",
            orthography: "",   // overridden later
            pronunciation: "", // overridden later
            orthography_base: "タベル",
            pronunciation_base: "タベル",
            origin: "和", // native Japanese
            word_subtype1: "*",
            word_subtype2: "*",
            word_subtype3: "*",
            alternate_form: "*",
        };
        assert_eq!(
            Details::new(&mut first_token("食べる")).unwrap(),
            Details {
                byte_end: "食べる".len(),
                orthography: "食べる",
                pronunciation: "食べる",
                ..taberu
            }
        );
        assert_eq!(
            Details::new(&mut first_token("たべる")).unwrap(),
            Details {
                byte_end: "たべる".len(),
                orthography: "たべる",
                pronunciation: "たべる",
                ..taberu
            }
        );

        let toukyou = Details {
            byte_start: 0,
            byte_end: 0,      // overridden later
            pos1: "名詞",     // noun
            pos2: "固有名詞", // proper noun
            pos3: "地名",     // place name
            pos4: "一般",     // general
            conjugation_type: "*",
            conjugation_form: "*",
            lexeme_form: "トウキョウ",
            lemma: "トウキョウ",
            orthography: "",   // overridden later
            pronunciation: "", // overridden later
            orthography_base: "トーキョー",
            pronunciation_base: "トーキョー",
            origin: "固", // proper noun
            word_subtype1: "*",
            word_subtype2: "*",
            word_subtype3: "*",
            alternate_form: "*",
        };
        assert_eq!(
            Details::new(&mut first_token("東京")).unwrap(),
            Details {
                byte_end: "東京".len(),
                orthography: "東京",
                pronunciation: "東京",
                ..toukyou
            }
        );
        assert_eq!(
            Details::new(&mut first_token("とうきょう")).unwrap(),
            Details {
                byte_end: "とうきょう".len(),
                orthography: "とうきょう",
                pronunciation: "とうきょう",
                ..toukyou
            }
        );
        assert_eq!(
            Details::new(&mut first_token("トウキョウ")).unwrap(),
            Details {
                byte_end: "トウキョウ".len(),
                orthography: "トウキョウ",
                pronunciation: "トウキョウ",
                ..toukyou
            }
        );
    }

    #[test]
    fn word_continuation() {
        // we want to test that, when deinflecting `full_text`,
        // we recognize that the first word in `full_text` is `word`
        #[track_caller]
        fn assert_split(word: &str, rem: &str) {
            let full_text = format!("{word}{rem}");
            let mut tokens = TOKENIZER.tokenize(&full_text).unwrap();
            let tokens = tokens
                .iter_mut()
                .map(Details::new)
                .collect::<Option<Vec<_>>>()
                .unwrap();

            let mut tokens = tokens.into_iter();
            // the たべる token in たべたい
            let last_lookahead = tokens.next().unwrap();

            // the たい in たべたい
            // (this is what we're testing)
            let word_last_token = tokens
                // in 消えてたじゃない, this stops at た *but includes it in the iterator*
                .take_while_inclusive(|token| !is_word_ending(token))
                // in 頼りなさげな目を, this stops at な目を *and doesn't include the remainder*
                .take_while(|token| is_word_continuation(&last_lookahead, token))
                .last();

            let word_end_byte =
                word_last_token.map_or(last_lookahead.byte_end, |details| details.byte_end);
            let (scanned_word, scanned_rem) = full_text.split_at(word_end_byte);
            assert_eq!(word, scanned_word);
            assert_eq!(rem, scanned_rem);
        }

        assert_split("食べる", "");
        assert_split("食べる", "あいう");
        assert_split("食べたい", "あいう");
        assert_split("食べなかった", "あいう");
        assert_split("騒がしかった", "こと");
        assert_split("大学", "とは");
        assert_split("頼りなさげな", "目を");

        // todo: this could technically be improved,
        // but technically the split isn't wrong here?
        // 〜いた (いる) *is* technically its own word
        // do we want to include て in the word as well? idk...
        // then we'd be including other particles like は, が etc.
        assert_split("叩きつけ", "ていた");

        // た is an auxiliary verb in terminal form
        // this test ensures that we stop scanning
        // after finding a terminal form verb
        assert_split("消えてた", "じゃない");
    }

    #[test]
    fn deinflect() {
        let deinflector = Lindera {
            tokenizer: TOKENIZER.clone(),
            lookahead: TOKEN_LOOKAHEAD,
        };

        assert_deinflects(&deinflector, sentence!(/ "食べる"), [deinf("食べる")]);

        let (text, start) = sentence!(/ "食べなかった");
        assert_deinflects(
            &deinflector,
            (text, start),
            [
                Deinflection::new(start, "食べなかった", "食べるないた"),
                Deinflection::new(start, "食べなかった", "食べなかった"),
                Deinflection::new(start, "食べなかっ", "食べるない"),
                Deinflection::new(start, "食べなかっ", "食べなかっ"),
                Deinflection::new(start, "食べなかった", "食べる"),
                Deinflection::new(start, "食べなかった", "食べ"),
            ],
        );

        let (text, start) = sentence!("あいう" / "食べなかった");
        assert_deinflects(
            &deinflector,
            (text, start),
            [
                Deinflection::new(start, "食べなかった", "食べるないた"),
                Deinflection::new(start, "食べなかった", "食べなかった"),
                Deinflection::new(start, "食べなかっ", "食べるない"),
                Deinflection::new(start, "食べなかっ", "食べなかっ"),
                Deinflection::new(start, "食べなかった", "食べる"),
                Deinflection::new(start, "食べなかった", "食べ"),
            ],
        );

        let (text, start) = sentence!(/ "東京大学");
        assert_deinflects(
            &deinflector,
            (text, start),
            [
                Deinflection::new(start, "東京大学", "トウキョウ大学"),
                Deinflection::new(start, "東京大学", "東京大学"),
                Deinflection::new(start, "東京", "トウキョウ"),
                Deinflection::new(start, "東京", "東京"),
            ],
        );

        // some token patterns might result in UNK tokens, like this trailing whitespace
        // here we test that we handle UNKs gracefully
        let (text, start) = sentence!(/ "ある。　");
        assert_deinflects(
            &deinflector,
            (text, start),
            [
                Deinflection::new(start, "ある。\u{3000}", "有る。\u{3000}"),
                Deinflection::new(start, "ある。\u{3000}", "ある。\u{3000}"),
                Deinflection::new(start, "ある。", "有る。"),
                Deinflection::new(start, "ある。", "ある。"),
                Deinflection::new(start, "ある", "有る"),
                Deinflection::new(start, "ある", "ある"),
            ],
        );

        // we want the deinflector to return words in the form
        // closest to what was originally written, rather than arbitrarily
        // choose to return the kanji or kana first
        let (text, start) = sentence!(/ "ありがとう");
        assert_deinflects(
            &deinflector,
            (text, start),
            [
                Deinflection::new(start, text, "ありがとう"),
                Deinflection::new(start, text, "有り難う"),
            ],
        );

        // here's the kanji form of ありがとう;
        // Lindera doesn't even give us the kana form, but that's fine.
        // it is what it is
        let (text, start) = sentence!(/ "有り難う");
        assert_deinflects(
            &deinflector,
            (text, start),
            [Deinflection::new(start, text, "有り難う")],
        );

        assert_deinflects(
            &deinflector,
            ("２０日", 0),
            [
                Deinflection::new(0, "２０日", "２０日"),
                Deinflection::new(0, "２０日", "二零日"),
                Deinflection::new(0, "２０", "２０"),
                Deinflection::new(0, "２０", "二零"),
                Deinflection::new(0, "２", "２"),
                Deinflection::new(0, "２", "二"),
            ],
        );

        assert_deinflects(
            &deinflector,
            ("20日", 0),
            [
                Deinflection::new(0, "２０日", "２０日"),
                Deinflection::new(0, "２０日", "二零日"),
                Deinflection::new(0, "２０", "２０"),
                Deinflection::new(0, "２０", "二零"),
                Deinflection::new(0, "２", "２"),
                Deinflection::new(0, "２", "二"),
            ],
        );
    }

    static TOKENIZER: LazyLock<Tokenizer> = LazyLock::new(|| {
        let dictionary = load_dictionary_from_kind(DictionaryKind::UniDic).unwrap();
        let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
        Tokenizer::new(segmenter)
    });

    fn first_token(text: &str) -> Token {
        TOKENIZER
            .tokenize(text)
            .unwrap()
            .into_iter()
            .next()
            .unwrap()
    }
}
