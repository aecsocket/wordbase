use {
    super::{Deinflection, Deinflector},
    unicode_segmentation::UnicodeSegmentation,
};

#[derive(Debug)]
pub struct Latin;

impl Deinflector for Latin {
    fn deinflect<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Deinflection<'a>> {
        let Some((start, word)) = text.unicode_word_indices().next() else {
            return Vec::new().into_iter();
        };

        vec![
            Deinflection::with_start(start, word),
            Deinflection::with_start(start, word.to_lowercase()),
            Deinflection::with_start(start, word.to_uppercase()),
        ]
        .into_iter()
    }
}

#[cfg(test)]
mod tests {
    use crate::deinflect::assert_deinflects;

    use super::*;

    #[test]
    fn transformations() {
        let deinflector = Latin;
        assert_deinflects(
            &deinflector,
            "hello",
            [Deinflection::new("hello"), Deinflection::new("HELLO")],
        );
        assert_deinflects(
            &deinflector,
            "HELLO",
            [Deinflection::new("hello"), Deinflection::new("HELLO")],
        );
        assert_deinflects(
            &deinflector,
            "Hello",
            [
                Deinflection::new("Hello"),
                Deinflection::new("hello"),
                Deinflection::new("HELLO"),
            ],
        );

        assert_deinflects(
            &deinflector,
            "hi world",
            [Deinflection::new("hi"), Deinflection::new("HI")],
        );
        assert_deinflects(
            &deinflector,
            "hi\nworld",
            [Deinflection::new("hi"), Deinflection::new("HI")],
        );
    }
}
