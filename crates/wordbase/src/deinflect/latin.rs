use {
    super::{Deinflection, Deinflector},
    unicode_segmentation::UnicodeSegmentation,
};

#[derive(Debug)]
pub struct Latin;

impl Deinflector for Latin {
    fn deinflect<'a>(&'a self, text: &'a str) -> impl Iterator<Item = Deinflection<'a>> {
        let Some(word) = text.unicode_words().next() else {
            return Vec::new().into_iter();
        };

        vec![
            Deinflection::new(word),
            Deinflection::new(word.to_lowercase()),
            Deinflection::new(word.to_uppercase()),
        ]
        .into_iter()
    }
}
