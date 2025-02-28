use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupConfig {
    pub max_request_len: u64,
    pub returns_html: bool,
}

impl Default for LookupConfig {
    fn default() -> Self {
        Self {
            max_request_len: 16,
            returns_html: false,
        }
    }
}
