use bytes::Bytes;
use derive_more::From;
use serde::{Deserialize, Serialize};

use crate::lookup::LookupConfig;

// client-to-server

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FromClient {
    pub request_id: RequestId,
    #[serde(flatten)]
    pub request: ClientRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RequestId(u64);

impl RequestId {
    #[must_use]
    pub const fn from_raw(id: u64) -> Self {
        Self(id)
    }

    #[must_use]
    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum ClientRequest {
    Lookup(Lookup),
    NewSentence(NewSentence),
    AddAnkiNote(AddAnkiNote),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lookup {
    pub text: String,
    pub wants_html: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSentence {
    pub process_path: String,
    pub sentence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddAnkiNote {
    pub text: String,
    pub image: Option<Bytes>,
    pub audio: Option<Bytes>,
}

// server-to-client

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FromServer {
    SyncLookupConfig(LookupConfig),
    NewSentence(NewSentence),
    Response {
        request_id: RequestId,
        #[serde(flatten)]
        response: Response,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
#[serde(tag = "type")]
pub enum Response {
    LookupInfo(#[serde(rename = "response")] Option<LookupInfo>),
    AddedAnkiNote,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LookupInfo {
    pub chars_scanned: u64,
    pub deinflected: String,
}
