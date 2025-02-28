use derive_more::From;
use serde::{Deserialize, Serialize};

use crate::lookup::LookupConfig;

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum Request {
    FetchLookupConfig,
    Lookup(LookupRequest),
    Deconjugate(DeconjugateRequest),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupRequest {
    pub text: String,
    pub wants_json: bool,
    pub wants_html: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeconjugateRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum Response {
    LookupConfig(LookupConfig),
    Lookup(LookupResponse),
    Deconjugate(DeconjugateResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupResponse {
    pub json: Lookup,
    pub html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeconjugateResponse {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lookup {
    pub chars_scanned: u64,
    pub entries: (), // TODO
}
