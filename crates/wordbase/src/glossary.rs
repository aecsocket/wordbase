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
