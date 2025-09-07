use {
    super::{Deinflection, Deinflector},
    itertools::Itertools,
    unicode_segmentation::UnicodeSegmentation,
};

#[derive(Debug)]
pub struct Latin;

impl Deinflector for Latin {
    fn deinflect<'a>(
        &'a self,
        sentence: &'a str,
        cursor: usize,
    ) -> impl Iterator<Item = Deinflection<'a>> {
        #[expect(
            clippy::unused_peekable,
            reason = "needed for `peeking_take_while` to be available"
        )]
        let mut words = sentence.unicode_word_indices().peekable();
        for _ in words.peeking_take_while(|(start, word)| (*start + word.len()) <= cursor) {}
        let Some((start, word)) = words.next() else {
            return Vec::new().into_iter();
        };

        vec![
            Deinflection::new(start, word, word),
            Deinflection::new(start, word, word.to_lowercase()),
            Deinflection::new(start, word, word.to_uppercase()),
        ]
        .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::deinflect::{
        sentence,
        tests::{assert_deinflects, deinf},
    };

    use super::*;

    #[test]
    fn transformations() {
        let deinflector = Latin;
        assert_deinflects(
            &deinflector,
            sentence!(/ "hello"),
            [deinf("hello"), deinf("HELLO")],
        );
        assert_deinflects(
            &deinflector,
            sentence!(/ "HELLO"),
            [deinf("hello"), deinf("HELLO")],
        );
        assert_deinflects(
            &deinflector,
            sentence!(/ "Hello"),
            [deinf("Hello"), deinf("hello"), deinf("HELLO")],
        );

        assert_deinflects(
            &deinflector,
            sentence!(/ "hi world"),
            [deinf("hi"), deinf("HI")],
        );
        assert_deinflects(
            &deinflector,
            sentence!(/ "hi\nworld"),
            [deinf("hi"), deinf("HI")],
        );

        let (text, start) = sentence!(/ "foo hello world");
        assert_deinflects(
            &deinflector,
            (text, start),
            [
                Deinflection::new(start, "foo", "foo"),
                Deinflection::new(start, "foo", "FOO"),
            ],
        );
        let (text, start, cursor) = sentence!(/ "f" / "oo hello world");
        assert_deinflects(
            &deinflector,
            (text, cursor),
            [
                Deinflection::new(start, "foo", "foo"),
                Deinflection::new(start, "foo", "FOO"),
            ],
        );

        let (text, cursor, start) = sentence!("foo" / " " / "hello world");
        assert_deinflects(
            &deinflector,
            (text, cursor),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );
        let (text, cursor, start) = sentence!("foo" / " " / "hello world");
        assert_deinflects(
            &deinflector,
            (text, cursor),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );
        let (text, start, cursor) = sentence!("foo " / "h" / "ello world");
        assert_deinflects(
            &deinflector,
            (text, cursor),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );
        let (text, start, cursor) = sentence!("foo " / "he" / "llo world");
        assert_deinflects(
            &deinflector,
            (text, cursor),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );

        assert_deinflects(&deinflector, sentence!("foo hello world" /), []);
    }

    #[test]
    fn on_non_latin() {
        let deinflector = Latin;
        assert_deinflects(&deinflector, sentence!(/ "店内に"), [deinf("店")]);

        let (text, start) = sentence!("店" / "内に");
        assert_deinflects(
            &deinflector,
            (text, start),
            [Deinflection::new(start, "内", "内")],
        );

        let (text, start) = sentence!("店内" / "に");
        assert_deinflects(
            &deinflector,
            (text, start),
            [Deinflection::new(start, "に", "に")],
        );
    }
}
