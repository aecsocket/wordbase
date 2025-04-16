//! Implementation logic for core crate types.
//!
//! This is in a separate module to keep `lib.rs` focused on documenting the
//! core API.

use std::{
    fmt::{self, Debug},
    mem,
};

use serde::Deserialize;

use crate::{DictionaryKind, DictionaryMeta, NormString, Term, TermPart};

impl DictionaryMeta {
    #[must_use]
    pub fn new(kind: DictionaryKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            version: None,
            description: None,
            url: None,
            attribution: None,
        }
    }
}

impl Term {
    #[must_use]
    pub fn new(headword: impl TermPart, reading: impl TermPart) -> Option<Self> {
        match (
            headword.try_into_non_empty_string(),
            reading.try_into_non_empty_string(),
        ) {
            (Some(headword), Some(reading)) => Some(Self::Full { headword, reading }),
            (Some(headword), None) => Some(Self::Headword { headword }),
            (None, Some(reading)) => Some(Self::Reading { reading }),
            (None, None) => None,
        }
    }

    #[must_use]
    pub fn from_headword<T: TermPart>(headword: T) -> T::IntoTerm {
        headword.into_term_with_headword()
    }

    #[must_use]
    pub fn from_reading<T: TermPart>(reading: T) -> T::IntoTerm {
        reading.into_term_with_reading()
    }

    #[must_use]
    pub fn headword(&self) -> Option<&NormString> {
        match self {
            Self::Full { headword, .. } | Self::Headword { headword } => Some(headword),
            Self::Reading { .. } => None,
        }
    }

    #[must_use]
    pub fn reading(&self) -> Option<&NormString> {
        match self {
            Self::Full { reading, .. } | Self::Reading { reading } => Some(reading),
            Self::Headword { .. } => None,
        }
    }

    #[must_use]
    pub fn headword_mut(&mut self) -> Option<&mut NormString> {
        match self {
            Self::Full { headword, .. } | Self::Headword { headword } => Some(headword),
            Self::Reading { .. } => None,
        }
    }

    #[must_use]
    pub fn reading_mut(&mut self) -> Option<&mut NormString> {
        match self {
            Self::Full { reading, .. } | Self::Reading { reading } => Some(reading),
            Self::Headword { .. } => None,
        }
    }

    pub fn set_headword(&mut self, new: NormString) -> Option<NormString> {
        match self {
            Self::Full { headword, .. } | Self::Headword { headword } => {
                Some(mem::replace(headword, new))
            }
            Self::Reading { reading } => {
                // CORRECTNESS: this non-empty string will never be accessed
                let reading = mem::replace(reading, NormString::new_unchecked(String::new()));
                *self = Self::Full {
                    headword: new,
                    reading,
                };
                None
            }
        }
    }

    pub fn set_reading(&mut self, new: NormString) -> Option<NormString> {
        match self {
            Self::Full { reading, .. } | Self::Reading { reading } => {
                Some(mem::replace(reading, new))
            }
            Self::Headword { headword } => {
                // CORRECTNESS: this non-empty string will never be accessed
                let headword = mem::replace(headword, NormString::new_unchecked(String::new()));
                *self = Self::Full {
                    headword,
                    reading: new,
                };
                None
            }
        }
    }

    #[must_use]
    pub fn take_headword(self) -> Option<NormString> {
        match self {
            Self::Full { headword, .. } | Self::Headword { headword } => Some(headword),
            Self::Reading { .. } => None,
        }
    }

    #[must_use]
    pub fn take_reading(self) -> Option<NormString> {
        match self {
            Self::Full { reading, .. } | Self::Reading { reading } => Some(reading),
            Self::Headword { .. } => None,
        }
    }
}

impl NormString {
    #[must_use]
    pub fn new(string: impl Into<String>) -> Option<Self> {
        let string: String = string.into();
        let trimmed = string.trim();
        if trimmed.is_empty() {
            return None;
        }

        if trimmed == &*string {
            Some(Self(string))
        } else {
            Some(Self(String::from(trimmed)))
        }
    }

    #[must_use]
    pub fn new_unchecked(string: String) -> Self {
        Self(string)
    }

    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Debug for NormString {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<'de> Deserialize<'de> for NormString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = NormString;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "non-empty string")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                NormString::new(v).ok_or_else(|| E::custom("string must be non-empty"))
            }

            fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                NormString::new(v).ok_or_else(|| E::custom("string must be non-empty"))
            }
        }

        deserializer.deserialize_string(Visitor)
    }
}

impl TermPart for NormString {
    type IntoTerm = Term;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        Some(self)
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        Term::Headword { headword: self }
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        Term::Reading { reading: self }
    }
}

impl TermPart for Option<NormString> {
    type IntoTerm = Option<Term>;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        self
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        self.map(TermPart::into_term_with_headword)
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        self.map(TermPart::into_term_with_reading)
    }
}

impl TermPart for String {
    type IntoTerm = Option<Term>;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        NormString::new(self)
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        NormString::new(self).into_term_with_headword()
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        NormString::new(self).into_term_with_reading()
    }
}

impl TermPart for Option<String> {
    type IntoTerm = Option<Term>;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        self.and_then(TermPart::try_into_non_empty_string)
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        self.and_then(TermPart::into_term_with_headword)
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        self.and_then(TermPart::into_term_with_reading)
    }
}

impl TermPart for &str {
    type IntoTerm = Option<Term>;

    fn try_into_non_empty_string(self) -> Option<NormString> {
        NormString::new(self)
    }

    fn into_term_with_headword(self) -> Self::IntoTerm {
        NormString::new(self).map(|headword| Term::Headword { headword })
    }

    fn into_term_with_reading(self) -> Self::IntoTerm {
        NormString::new(self).map(|reading| Term::Reading { reading })
    }
}

#[cfg(feature = "poem-openapi")]
const _: () = {
    use poem_openapi::types::{ParseError, ParseFromJSON, ParseResult};

    impl ParseFromJSON for NormString {
        fn parse_from_json(value: Option<serde_json::Value>) -> ParseResult<Self> {
            let raw = String::parse_from_json(value).map_err(ParseError::propagate)?;
            Self::new(raw).ok_or_else(|| ParseError::custom("string must be non-empty"))
        }
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term_api() {
        const TEST: &str = "test";

        let test = NormString::new(TEST).unwrap();

        assert_eq!(Term::from_headword(""), None);
        assert_eq!(
            Term::from_headword(TEST),
            Some(Term::Headword {
                headword: test.clone(),
            })
        );
        let mut term = Term::from_headword(test.clone());
        assert_eq!(
            term,
            Term::Headword {
                headword: test.clone()
            }
        );
        term.set_reading(test.clone());
        assert_eq!(
            term,
            Term::Full {
                headword: test.clone(),
                reading: test.clone()
            }
        );

        assert_eq!(Term::from_reading(""), None);
        assert_eq!(
            Term::from_reading(TEST),
            Some(Term::Reading {
                reading: test.clone()
            })
        );
        let mut term = Term::from_reading(test.clone());
        assert_eq!(
            term,
            Term::Reading {
                reading: test.clone(),
            }
        );
        term.set_headword(test.clone());
        assert_eq!(
            term,
            Term::Full {
                headword: test.clone(),
                reading: test
            }
        );
    }
}
