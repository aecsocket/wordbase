#![doc = include_str!("../README.md")]

#[allow(unused_macros, reason = "used in tests")]
macro_rules! sentence {
    (/ $a:literal) => {{
        let text = $a;
        (text, 0usize)
    }};
    ($a:literal /) => {{
        let text = $a;
        (text, $a.len())
    }};
    ($a:literal / $b:literal) => {{
        let text = concat!($a, $b);
        (text, $a.len())
    }};
    (/ $a:literal / $b:literal) => {{
        let text = concat!($a, $b);
        (text, 0usize, $a.len())
    }};
    ($a:literal / $b:literal / $c:literal) => {{
        let text = concat!($a, $b, $c);
        (text, $a.len(), concat!($a, $b).len())
    }};
}

mod identity;
mod latin;

use wordbase_core::deinflect::Deinflector;
pub use {identity::Identity, latin::Latin};

/// All [`Deinflector`]s from this crate.
pub const DEINFLECTORS: &[&dyn Deinflector] = &[&Identity, &Latin];

#[cfg(test)]
mod tests {
    use {
        indexmap::IndexSet,
        wordbase_core::deinflect::{Deinflection, Deinflector},
    };

    #[track_caller]
    pub fn assert_deinflects<'t>(
        deinflector: &impl Deinflector,
        (sentence, cursor): (&str, usize),
        expected: impl IntoIterator<Item = Deinflection<'t>>,
    ) {
        assert_eq!(
            deinflector
                .deinflect(sentence, cursor)
                .unwrap()
                .into_iter()
                .collect::<IndexSet<_>>(),
            expected.into_iter().collect::<IndexSet<_>>(),
        );
    }

    #[test]
    fn sentence() {
        assert_eq!(sentence!(/ "hello"), ("hello", 0usize));
        assert_eq!(sentence!("hello" /), ("hello", "hello".len()));
        assert_eq!(
            sentence!("hello " / "world"),
            ("hello world", "hello ".len())
        );
        assert_eq!(
            sentence!(/ "hello " / "world"),
            ("hello world", 0usize, "hello ".len())
        );
        assert_eq!(
            sentence!("hello " / "and" / " goodbye"),
            ("hello and goodbye", "hello ".len(), "hello and".len()),
        );
    }
}
