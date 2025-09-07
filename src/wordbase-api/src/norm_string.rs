use std::str::FromStr;

use derive_more::{Debug, Deref, Display, Error};
use serde::{Deserialize, Serialize};

/// Normalized string buffer.
///
/// This type is guaranteed to be a non-empty string with no trailing or leading
/// whitespace.
#[derive(Debug, Display, Clone, PartialEq, Eq, Hash, Deref, Serialize)]
#[cfg_attr(
    feature = "poem",
    derive(poem_openapi::NewType),
    oai(from_json = false, from_parameter = false, from_multipart = false)
)]
#[debug("{_0:?}")]
pub struct NormString(String);

#[cfg(feature = "uniffi")]
uniffi::custom_type!(NormString, String);

impl NormString {
    /// Attempts to create a new value from an existing string.
    ///
    /// If the string is empty, returns [`None`].
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

    /// Creates a new value from an existing string without checking for
    /// emptiness.
    ///
    /// # Correctness
    ///
    /// The trimmed string must not be empty.
    #[must_use]
    pub const fn new_unchecked(string: String) -> Self {
        Self(string)
    }

    /// Takes the underlying string.
    #[must_use]
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<NormString> for String {
    fn from(value: NormString) -> Self {
        value.0
    }
}

impl PartialEq<NormString> for &str {
    fn eq(&self, other: &NormString) -> bool {
        *self == other.0
    }
}

/// Attempted to turn a string into a [`NormString`], but the string was empty.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Error)]
#[display("string empty")]
pub struct StringEmpty;

impl TryFrom<&str> for NormString {
    type Error = StringEmpty;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(StringEmpty)
    }
}

impl TryFrom<String> for NormString {
    type Error = StringEmpty;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value).ok_or(StringEmpty)
    }
}

impl FromStr for NormString {
    type Err = StringEmpty;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s).ok_or(StringEmpty)
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

#[cfg(feature = "poem")]
const _: () = {
    use {
        poem::web::Field,
        poem_openapi::types::{
            ParseError, ParseFromJSON, ParseFromMultipartField, ParseFromParameter, ParseResult,
        },
    };

    impl ParseFromJSON for NormString {
        fn parse_from_json(value: Option<serde_json::Value>) -> ParseResult<Self> {
            let raw = String::parse_from_json(value).map_err(ParseError::propagate)?;
            Self::new(raw).ok_or_else(|| ParseError::custom(StringEmpty))
        }
    }

    impl ParseFromParameter for NormString {
        fn parse_from_parameter(value: &str) -> ParseResult<Self> {
            let raw = String::parse_from_parameter(value).map_err(ParseError::propagate)?;
            Self::new(raw).ok_or_else(|| ParseError::custom(StringEmpty))
        }
    }

    impl ParseFromMultipartField for NormString {
        async fn parse_from_multipart(field: Option<Field>) -> ParseResult<Self> {
            let raw = String::parse_from_multipart(field)
                .await
                .map_err(ParseError::propagate)?;
            Self::new(raw).ok_or_else(|| ParseError::custom(StringEmpty))
        }
    }
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ns_api() {
        assert!(NormString::new("").is_none());
        assert_eq!("foo", NormString::new("foo").unwrap());
        assert_eq!("foo", NormString::new(" foo").unwrap());
        assert_eq!("foo", NormString::new("  foo").unwrap());
        assert_eq!("foo", NormString::new("foo ").unwrap());
        assert_eq!("foo", NormString::new("foo  ").unwrap());
        assert_eq!("foo", NormString::new(" foo ").unwrap());
        assert_eq!("foo", NormString::new("  foo  ").unwrap());

        "".parse::<NormString>().unwrap_err();
        assert_eq!("foo", "foo".parse::<NormString>().unwrap());
    }
}
