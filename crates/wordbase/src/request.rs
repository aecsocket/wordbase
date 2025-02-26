use derive_more::From;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum Request {
    #[serde(rename = "WordbasePing")]
    Ping,
    Lookup(Lookup),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lookup {
    pub text: String,
}
