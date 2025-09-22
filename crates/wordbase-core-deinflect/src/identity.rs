use {
    eyre::{Result, eyre},
    wordbase_core::deinflect::{Deinflection, Deinflector},
};

/// Returns the given text verbatim as a lemma.
#[derive(Debug)]
pub struct Identity;

impl Deinflector for Identity {
    fn deinflect<'t>(&self, sentence: &'t str, cursor: usize) -> Result<Vec<Deinflection<'t>>> {
        let rest = sentence.get(cursor..).ok_or_else(|| {
            eyre!(
                "cursor {cursor} out of bounds for string of length {}",
                sentence.len()
            )
        })?;

        Ok(vec![Deinflection::new(cursor, rest, rest)])
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::tests::assert_deinflects};

    #[test]
    fn identity() {
        let (text, start) = sentence!(/ "hello world");
        assert_deinflects(
            &Identity,
            (text, start),
            [Deinflection::new(start, "hello world", "hello world")],
        );
        let (text, start) = sentence!("hello " / "world");
        assert_deinflects(
            &Identity,
            (text, start),
            [Deinflection::new(start, "world", "world")],
        );
        let (text, start) = sentence!("hello world " /);
        assert_deinflects(&Identity, (text, start), []);
    }
}
