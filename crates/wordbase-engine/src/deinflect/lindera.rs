use lindera::token::Token;

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
    lemma: &'a str,
    reading: &'a str,
    pronunciation: &'a str,
    pronunciation_base_form: &'a str,
    spoken_base_form: &'a str,
    spoken_form: &'a str,
    word_type: &'a str,
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
            lemma: details.next()?,
            reading: details.next()?,
            pronunciation: details.next()?,
            pronunciation_base_form: details.next()?,
            spoken_base_form: details.next()?,
            spoken_form: details.next()?,
            word_type: details.next()?,
            word_subtype1: details.next()?,
            word_subtype2: details.next()?,
            word_subtype3: details.next()?,
            alternate_form: details.next()?,
        })
    }
}

fn is_word_ending(token: &Details) -> bool {
    // terminal form
    matches!(token.conjugation_form, "終止形-一般")
}

fn is_word_continuation(last_lookahead: &Details, token: &Details) -> bool {
    match last_lookahead.pos1 {
        // verb
        "動詞" => {
            matches!(token.pos1, "助動詞") // auxiliary verb
        }
        // adjective
        "形容詞" => {
            matches!(token.pos1, "接尾辞") // suffix
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use itertools::Itertools;
    use lindera::{
        dictionary::{DictionaryKind, load_dictionary_from_kind},
        mode::Mode,
        segmenter::Segmenter,
        tokenizer::Tokenizer,
    };

    use super::*;

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
            lemma: "タベル",
            reading: "食べる",
            pronunciation: "", // overridden later
            pronunciation_base_form: "タベル",
            spoken_base_form: "", // overridden later
            spoken_form: "タベル",
            word_type: "和", // native Japanese
            word_subtype1: "*",
            word_subtype2: "*",
            word_subtype3: "*",
            alternate_form: "*",
        };
        assert_eq!(
            Details::new(&mut first_token("食べる")).unwrap(),
            Details {
                byte_end: "食べる".len(),
                pronunciation: "食べる",
                spoken_base_form: "食べる",
                ..taberu
            }
        );
        assert_eq!(
            Details::new(&mut first_token("たべる")).unwrap(),
            Details {
                byte_end: "たべる".len(),
                pronunciation: "たべる",
                spoken_base_form: "たべる",
                ..taberu
            }
        );
    }

    #[test]
    fn word_continuation() {
        // we want to test that, when deinflecting `full_text`,
        // we recognize that the first word in `full_text` is `word`
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
        assert_split("大学", "とは");
        assert_split("頼りなさげ", "な目を");

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
