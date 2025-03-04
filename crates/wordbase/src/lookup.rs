use serde::{Deserialize, Serialize};

use crate::dict::ExpressionEntry;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupConfig {
    pub max_request_len: u16,
}

impl Default for LookupConfig {
    fn default() -> Self {
        Self {
            max_request_len: 16,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupInfo {
    pub lemma: String,
    pub expressions: Vec<ExpressionEntry>,
}
