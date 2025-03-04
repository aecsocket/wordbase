use serde::{Deserialize, Serialize};

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
}
