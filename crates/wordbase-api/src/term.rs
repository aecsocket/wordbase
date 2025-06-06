use std::mem;

use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

use crate::NormString;

/// Key for [`Record`]s in a [`Dictionary`].
///
/// A term consists of at least one of a headword or a reading.
/// If a term part is present, it is guaranteed to be non-empty,
/// enforced by [`NormString`].
///
/// - headword: the canonical dictionary form of a word
/// - reading: disambiguates the headword between entries which use the same
///   headword but for different words (i.e. Japanese kana reading)
///
/// For languages without the concept of a reading, only the headword should be
/// specified.
///
/// [`Record`]: crate::Record
/// [`Dictionary`]: crate::Dictionary
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash)]
pub enum Term {
    /// Headword only.
    #[display("{_0}")]
    Headword(NormString),
    /// Reading only.
    #[display("{_0}")]
    Reading(NormString),
    /// Headword and reading.
    #[display("{_0} ({_1})")]
    Full(NormString, NormString),
}

impl Term {
    /// Creates a value from a headword/reading pair.
    ///
    /// If both are not present or empty, returns [`None`].
    #[must_use]
    pub fn from_parts(
        headword: Option<impl TryInto<NormString>>,
        reading: Option<impl TryInto<NormString>>,
    ) -> Option<Self> {
        match (
            headword.and_then(|s| s.try_into().ok()),
            reading.and_then(|s| s.try_into().ok()),
        ) {
            (Some(headword), Some(reading)) => Some(Self::Full(headword, reading)),
            (Some(headword), None) => Some(Self::Headword(headword)),
            (None, Some(reading)) => Some(Self::Reading(reading)),
            (None, None) => None,
        }
    }

    /// Creates a value from a headword and reading.
    pub fn from_full(
        headword: impl TryInto<NormString>,
        reading: impl TryInto<NormString>,
    ) -> Option<Self> {
        Self::from_parts(Some(headword), Some(reading))
    }

    /// Creates a value from only a headword.
    pub fn from_headword(headword: impl TryInto<NormString>) -> Option<Self> {
        Self::from_parts(Some(headword), None::<NormString>)
    }

    /// Creates a value from only a headword.
    pub fn from_reading(reading: impl TryInto<NormString>) -> Option<Self> {
        Self::from_parts(None::<NormString>, Some(reading))
    }

    /// Gets the underlying headword and reading from this term.
    #[must_use]
    pub fn into_parts(self) -> (Option<NormString>, Option<NormString>) {
        match self {
            Self::Headword(headword) => (Some(headword), None),
            Self::Reading(reading) => (None, Some(reading)),
            Self::Full(headword, reading) => (Some(headword), Some(reading)),
        }
    }

    /// Gets a reference to the headword, if present.
    #[must_use]
    pub const fn headword(&self) -> Option<&NormString> {
        match self {
            Self::Headword(headword) | Self::Full(headword, _) => Some(headword),
            Self::Reading(_) => None,
        }
    }

    /// Gets a reference to the reading, if present.
    #[must_use]
    pub const fn reading(&self) -> Option<&NormString> {
        match self {
            Self::Reading(reading) | Self::Full(_, reading) => Some(reading),
            Self::Headword(_) => None,
        }
    }

    /// Gets a mutable reference to the headword, if present.
    #[must_use]
    pub fn headword_mut(&mut self) -> Option<&mut NormString> {
        match self {
            Self::Headword(headword) | Self::Full(headword, _) => Some(headword),
            Self::Reading(_) => None,
        }
    }

    /// Gets a mutable reference to the reading, if present.
    #[must_use]
    pub fn reading_mut(&mut self) -> Option<&mut NormString> {
        match self {
            Self::Reading(reading) | Self::Full(_, reading) => Some(reading),
            Self::Headword(_) => None,
        }
    }

    /// Replaces this value with one that has [`Term::headword`] set to the
    /// given value.
    pub fn set_headword(&mut self, headword: NormString) {
        let (_, reading) = mem::replace(self, DUMMY).into_parts();
        *self = if let Some(reading) = reading {
            Self::Full(headword, reading)
        } else {
            Self::Headword(headword)
        };
    }

    /// Replaces this value with one that has [`Term::reading`] set to the
    /// given value.
    pub fn set_reading(&mut self, reading: NormString) {
        let (headword, _) = mem::replace(self, DUMMY).into_parts();
        *self = if let Some(headword) = headword {
            Self::Full(headword, reading)
        } else {
            Self::Reading(reading)
        };
    }
}

const DUMMY: Term = Term::Headword(NormString::new_unchecked(String::new()));

impl<H: TryInto<NormString>, R: TryInto<NormString>> TryFrom<(Option<H>, Option<R>)> for Term {
    type Error = NoHeadwordOrReading;

    fn try_from((headword, reading): (Option<H>, Option<R>)) -> Result<Self, Self::Error> {
        Self::from_parts(headword, reading).ok_or(NoHeadwordOrReading)
    }
}

impl<H: TryInto<NormString>, R: TryInto<NormString>> TryFrom<(H, R)> for Term {
    type Error = NoHeadwordOrReading;

    fn try_from((headword, reading): (H, R)) -> Result<Self, Self::Error> {
        Self::from_parts(Some(headword), Some(reading)).ok_or(NoHeadwordOrReading)
    }
}

/// Attempted to create a [`Term`] from a headword/reading pair, but both were
/// not present or empty.
#[derive(Debug, Display, Clone, Default, Error)]
#[display("no headword or reading")]
pub struct NoHeadwordOrReading;

const _: () = {
    #[derive(Serialize)]
    pub struct TermSerial<'a> {
        headword: Option<&'a NormString>,
        reading: Option<&'a NormString>,
    }

    #[derive(Deserialize)]
    pub struct TermDeserial {
        headword: Option<NormString>,
        reading: Option<NormString>,
    }

    impl Serialize for Term {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            TermSerial {
                headword: self.headword(),
                reading: self.reading(),
            }
            .serialize(serializer)
        }
    }

    impl<'de> Deserialize<'de> for Term {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let TermDeserial { headword, reading } = TermDeserial::deserialize(deserializer)?;
            Self::try_from((headword, reading)).map_err(serde::de::Error::custom)
        }
    }
};

#[cfg(feature = "uniffi")]
const _: () = {
    #[derive(uniffi::Record)]
    pub struct TermFfi {
        headword: Option<NormString>,
        reading: Option<NormString>,
    }

    uniffi::custom_type!(Term, TermFfi, {
        lower: |term| {
            let (headword, reading) = term.into_parts();
            TermFfi { headword, reading }
        },
        try_lift: |ffi| Ok(Term::try_from((ffi.headword, ffi.reading))?),
    });
};

#[cfg(test)]
mod tests {
    use super::*;

    fn ns(s: &str) -> NormString {
        NormString::new(s).unwrap()
    }

    #[test]
    fn term_api() {
        assert!(Term::from_parts(None::<NormString>, None::<NormString>).is_none());
        assert!(Term::from_full("", "").is_none());
        assert!(Term::from_headword("").is_none());
        assert!(Term::from_reading("").is_none());

        assert_eq!(
            Term::from_full("hello", "world").unwrap(),
            Term::Full(ns("hello"), ns("world")),
        );
        assert_eq!(
            Term::from_full("foo", "").unwrap(),
            Term::Headword(ns("foo")),
        );
        assert_eq!(
            Term::from_full("", "foo").unwrap(),
            Term::Reading(ns("foo")),
        );

        assert_eq!(
            Term::from_headword("foo").unwrap(),
            Term::Headword(ns("foo")),
        );
        assert_eq!(Term::from_reading("foo").unwrap(), Term::Reading(ns("foo")),);
    }
}
