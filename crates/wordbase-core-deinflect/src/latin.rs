use {
    eyre::Result,
    itertools::Itertools,
    unicode_segmentation::UnicodeSegmentation,
    wordbase_core::deinflect::{Deinflection, Deinflector},
};

/// Uses [`UnicodeSegmentation`] to split the text into words and returns each
/// word, its lowercase, and uppercase variants as lemmas.
#[derive(Debug)]
pub struct Latin;

impl Deinflector for Latin {
    fn deinflect<'t>(&self, sentence: &'t str, cursor: usize) -> Result<Vec<Deinflection<'t>>> {
        #[expect(
            clippy::unused_peekable,
            reason = "needed for `peeking_take_while` to be available"
        )]
        let mut words = sentence.unicode_word_indices().peekable();
        for _ in words.peeking_take_while(|(start, word)| (*start + word.len()) <= cursor) {}
        if let Some((start, word)) = words.next() {
            Ok(vec![
                Deinflection::new(start, word, word),
                Deinflection::new(start, word, word.to_lowercase()),
                Deinflection::new(start, word, word.to_uppercase()),
            ])
        } else {
            Ok(vec![])
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::Latin, crate::tests::assert_deinflects, wordbase_core::deinflect::Deinflection};

    #[test]
    fn transformations() {
        let (text, start) = sentence!(/ "hello");
        assert_deinflects(
            &Latin,
            (text, start),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );
        let (text, start) = sentence!(/ "HELLO");
        assert_deinflects(
            &Latin,
            (text, start),
            [
                Deinflection::new(start, "HELLO", "hello"),
                Deinflection::new(start, "HELLO", "HELLO"),
            ],
        );
        let (text, start) = sentence!(/ "Hello");
        assert_deinflects(
            &Latin,
            (text, start),
            [
                Deinflection::new(start, "Hello", "Hello"),
                Deinflection::new(start, "Hello", "hello"),
                Deinflection::new(start, "Hello", "HELLO"),
            ],
        );

        let (text, start) = sentence!(/ "hi world");
        assert_deinflects(
            &Latin,
            (text, start),
            [
                Deinflection::new(start, "hi", "hi"),
                Deinflection::new(start, "hi", "HI"),
            ],
        );
        let (text, start) = sentence!(/ "hi\nworld");
        assert_deinflects(
            &Latin,
            (text, start),
            [
                Deinflection::new(start, "hi", "hi"),
                Deinflection::new(start, "hi", "HI"),
            ],
        );

        let (text, start) = sentence!(/ "foo hello world");
        assert_deinflects(
            &Latin,
            (text, start),
            [
                Deinflection::new(start, "foo", "foo"),
                Deinflection::new(start, "foo", "FOO"),
            ],
        );
        let (text, start, cursor) = sentence!(/ "f" / "oo hello world");
        assert_deinflects(
            &Latin,
            (text, cursor),
            [
                Deinflection::new(start, "foo", "foo"),
                Deinflection::new(start, "foo", "FOO"),
            ],
        );

        let (text, cursor, start) = sentence!("foo" / " " / "hello world");
        assert_deinflects(
            &Latin,
            (text, cursor),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );
        let (text, cursor, start) = sentence!("foo" / " " / "hello world");
        assert_deinflects(
            &Latin,
            (text, cursor),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );
        let (text, start, cursor) = sentence!("foo " / "h" / "ello world");
        assert_deinflects(
            &Latin,
            (text, cursor),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );
        let (text, start, cursor) = sentence!("foo " / "he" / "llo world");
        assert_deinflects(
            &Latin,
            (text, cursor),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );

        assert_deinflects(&Latin, sentence!("foo hello world" /), []);
    }

    #[test]
    fn on_non_latin() {
        let (text, start) = sentence!(/ "店内に");
        assert_deinflects(
            &Latin,
            (text, start),
            [Deinflection::new(start, "店", "店")],
        );
        let (text, start) = sentence!("店" / "内に");
        assert_deinflects(
            &Latin,
            (text, start),
            [Deinflection::new(start, "内", "内")],
        );
        let (text, start) = sentence!("店内" / "に");
        assert_deinflects(
            &Latin,
            (text, start),
            [Deinflection::new(start, "に", "に")],
        );
    }
}
