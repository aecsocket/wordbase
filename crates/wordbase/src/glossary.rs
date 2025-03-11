//! [Record][record] kinds which provide a definition for a [term].
//!
//! This is the record kind which you are probably most interested.
//! A glossary defines what a term actually means in human-readable terms - the
//! natural meaning of "dictionary entry". However, the content is left
//! deliberately undefined, and it is up to the dictionary to fill out what it
//! wants for its glossaries. Some dictionaries are monolingual, and may provide
//! a definition in the dictionary's own language. Others are bilingual, and
//! provide a translated meaning in the reader's native language.
//!
//! Glossaries are complicated - there are many different formats of glossaries
//! in the wild, and each has their own format which they store content in,
//! sometimes bespoke. The `pyglossary` project has a [list of supported
//! glossary formats][formats] which is a good starting place to explore what
//! formats exist. But due to this fragmentation, we cannot sanely define a
//! single format to use for all glossaries, as we cannot guarantee that you
//! can convert from one to another.
//!
//! Instead of defining a single universal glossary format, we support
//! glossaries in multiple formats. It is up to you to use the format which is
//! most convenient for you if it is present, or fallback to a different format
//! (potentially to multiple different formats).
//!
//! [record]: crate::Record
//! [term]: crate::Term
//! [formats]: https://github.com/ilius/pyglossary/#supported-formats

use {
    derive_more::{Deref, DerefMut},
    serde::{Deserialize, Serialize},
};

/// Definition of a [term] in plain text format.
///
/// This is the simplest glossary format, with no frills. Just text.
///
/// [term]: crate::Term
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deref, DerefMut, Serialize, Deserialize)]
pub struct PlainText(pub String);

/// Definition of a [term] in HTML format.
///
/// This is a well-supported format which is common in many dictionaries,
/// and can be easily rendered by many clients (as long as you have access
/// to a [`WebView`] widget, or are rendering inside a browser).
///
/// [term]: crate::Term
/// [`WebView`]: https://en.wikipedia.org/wiki/WebView
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deref, DerefMut, Serialize, Deserialize)]
pub struct Html(pub String);
