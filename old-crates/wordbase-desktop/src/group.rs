// TODO: this should be moved out to a more core crate
// idk exactly where though yet

use {
    derive_more::{Deref, DerefMut},
    std::{
        borrow::Borrow,
        hash::{Hash, Hasher},
    },
    wordbase::Term,
};

#[derive(Debug, Clone, Copy, Deref, DerefMut)]
pub struct Grouping<T: Borrow<Term>>(pub T);

fn bucket_of(term: &Term) -> &str {
    match term {
        Term::Full {
            headword,
            reading: _,
        }
        | Term::Headword { headword } => headword,
        Term::Reading { reading } => reading,
    }
}

impl<T: Borrow<Term>> Hash for Grouping<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        bucket_of(self.0.borrow()).hash(state);
    }
}

impl<T: Borrow<Term>> PartialEq for Grouping<T> {
    fn eq(&self, other: &Self) -> bool {
        let this_bucket = bucket_of(self.0.borrow());
        let other_bucket = bucket_of(other.0.borrow());
        this_bucket == other_bucket
    }
}

impl<T: Borrow<Term>> Eq for Grouping<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records() {
        assert_ne!(
            Grouping(Term::new("見る", "みる").unwrap()),
            Grouping(Term::from_headword("みる").unwrap()),
        );
        assert_eq!(
            Grouping(Term::new("薄情", "はくじょう").unwrap()),
            Grouping(Term::from_headword("薄情").unwrap()),
        );
    }
}
