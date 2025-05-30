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
        for _ in words.peeking_take_while(|(start, word)| (*start + word.len()) < cursor) {}
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
    use crate::deinflect::tests::{assert_deinflects, deinf};

    use super::*;

    #[test]
    fn transformations() {
        let deinflector = Latin;
        assert_deinflects(&deinflector, "hello", 0, [deinf("hello"), deinf("HELLO")]);
        assert_deinflects(&deinflector, "HELLO", 0, [deinf("hello"), deinf("HELLO")]);
        assert_deinflects(
            &deinflector,
            "Hello",
            0,
            [deinf("Hello"), deinf("hello"), deinf("HELLO")],
        );

        assert_deinflects(&deinflector, "hi world", 0, [deinf("hi"), deinf("HI")]);
        assert_deinflects(&deinflector, "hi\nworld", 0, [deinf("hi"), deinf("HI")]);

        assert_deinflects(
            &deinflector,
            "foo hello world",
            0,
            [
                Deinflection::new(0, "foo", "foo"),
                Deinflection::new(0, "foo", "FOO"),
            ],
        );
        assert_deinflects(
            &deinflector,
            "foo hello world",
            "f".len(),
            [
                Deinflection::new(0, "foo", "foo"),
                Deinflection::new(0, "foo", "FOO"),
            ],
        );
        assert_deinflects(
            &deinflector,
            "foo hello world",
            "foo".len(),
            [
                Deinflection::new(0, "foo", "foo"),
                Deinflection::new(0, "foo", "FOO"),
            ],
        );

        let start = "foo ".len();
        assert_deinflects(
            &deinflector,
            "foo hello world",
            "foo ".len(),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );
        assert_deinflects(
            &deinflector,
            "foo hello world",
            "foo h".len(),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );
        assert_deinflects(
            &deinflector,
            "foo hello world",
            "foo he".len(),
            [
                Deinflection::new(start, "hello", "hello"),
                Deinflection::new(start, "hello", "HELLO"),
            ],
        );

        let start = "foo hello ".len();
        assert_deinflects(
            &deinflector,
            "foo hello world",
            "foo hello world".len(),
            [
                Deinflection::new(start, "world", "world"),
                Deinflection::new(start, "world", "WORLD"),
            ],
        );
    }
}
