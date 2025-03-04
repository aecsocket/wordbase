use serde::{Deserialize, Serialize};

use crate::dict::ExpressionEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedConfig {
    pub max_lookup_len: u16,
}

impl Default for SharedConfig {
    fn default() -> Self {
        Self { max_lookup_len: 16 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupInfo {
    pub lemma: String,
    pub expressions: Vec<ExpressionEntry>,
}
