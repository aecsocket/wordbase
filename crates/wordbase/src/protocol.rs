use derive_more::From;
use serde::{Deserialize, Serialize};

use crate::lookup::LookupConfig;

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum Request {
    FetchLookupConfig,
    Lookup(LookupRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupRequest {
    pub text: String,
    pub wants_html: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Response {
    LookupConfig(LookupConfig),
    Lookup { response: Option<LookupResponse> },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResponse {
    pub raw: Lookup,
    pub html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lookup {
    pub chars_scanned: u64,
    pub entries: String, // TODO
}
