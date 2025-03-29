use bytes::Bytes;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forvo {
    pub username: String,
    pub audio: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jpod {
    pub audio: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nhk16 {
    pub audio: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shinmeikai8 {
    pub audio: Bytes,
}
