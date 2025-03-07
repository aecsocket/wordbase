//! Protocol for clients performing a lookup query against the server.

use serde::{Deserialize, Serialize};

use crate::{DictionaryId, Frequency, Glossary, Term, lang::jp};

/// Configuration for [lookup operations] shared between a Wordbase client and
/// server.
///
/// [lookup operations]: crate::protocol::FromClient::Lookup
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupConfig {
    /// Maximum length, in **characters** (not bytes), that [`Lookup::text`] is
    /// allowed to be.
    ///
    /// The maximum length of lookup requests is capped to avoid overloading the
    /// server with extremely large lookup requests. Clients must respect the
    /// server's configuration and not send any lookups longer than this,
    /// otherwise the server will return an error.
    ///
    /// [`Lookup::text`]: crate::protocol::FromClient::Lookup::text
    pub max_request_len: u64,
}

impl Default for LookupConfig {
    fn default() -> Self {
        Self {
            max_request_len: 16,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LookupEntry {
    pub lemma: String,
    pub terms: Vec<(DictionaryId, Term, LookupTerm)>,
}

/// Lookup response for a single [term] found in a single [dictionary].
///
/// # Language-specific details
///
/// While this type contains data on language-agnostic details such as
/// [glossaries], it also contains data which is specific to certain languages.
///
/// This type is marked as `#[non_exhaustive]` to allow adding new lookup
/// results in the future without breaking existing code.
///
/// [term]: crate::Term
/// [dictionary]: crate::Dictionary
/// [glossaries]: crate::Glossary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LookupTerm {
    /// Glossaries for this term.
    pub glossaries: Vec<Glossary>,
    /// Frequencies for this term.
    pub frequencies: Vec<Frequency>,
    /// **Japanese** pitch accent data.
    pub jp_pitches: Vec<jp::Pitch>,
}
