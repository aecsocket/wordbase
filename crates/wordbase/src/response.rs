use alloc::string::String;
use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum Response {
    #[serde(rename = "WordbasePong")]
    Pong(Pong),
    Lookup(Lookup),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pong {
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lookup {
    pub chars_matched: u64,
    pub entries: String, // TODO
}
