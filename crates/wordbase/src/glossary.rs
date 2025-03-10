//! # Glossaries
//!
//! The record kind which you are probably most interested in is the *glossary*,
//! which defines what a term actually means in human-readable terms - the
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
// TODO: is this a good idea?
//! # Dynamic records
//!
//! Some records, such as certain kinds of glossary records, may be *dynamic*.
//! This means that their contents aren't actually stored in the server's
//! database, but are instead computed on-the-fly from the data that it *does*
//! have when you make your request. It may even provide or omit entire records
//! based on what record kinds you request.
//!
//! For example, the server may store a [Yomitan structured content][content]
//! record internally for a given term. If you request a [`YomitanGlossary`],
//! the server will provide you with this, but will *not* provide a
//! [`GlossaryHtml`] - you can compute that yourself from the structured content
//! you're given, and render it in your own way. However, if you don't request a
//! [`YomitanGlossary`], the server falls back to generating HTML by itself and
//! sending you the result - it will assume that you don't know what a Yomitan
//! glossary is, but still wants to provide a result.
//!
//! [formats]: https://github.com/ilius/pyglossary/#supported-formats

use derive_more::{Deref, DerefMut};
use serde::{Deserialize, Serialize};

/// Definition of a [term] in plain text format.
///
/// This is the simplest glossary format, and should be used as a fallback
/// if there is no other way to express your glossary content. Similarly,
/// clients should only use this as a fallback source for rendering.
///
/// [term]: crate::Term
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deref, DerefMut, Serialize, Deserialize)]
pub struct PlainText(pub String);

/// Fallback version of [`PlainText`].
///
/// See [`glossary`](self).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deref, DerefMut, Serialize, Deserialize)]
pub struct PlainTextFallback(pub String);

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

/// Fallback version of [`Html`].
///
/// See [`glossary`](self).
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deref, DerefMut, Serialize, Deserialize)]
pub struct HtmlFallback(pub String);
