use bytes::Bytes;
use serde::{Deserialize, Serialize};

use crate::NonEmptyString;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Forvo {
    pub username: String,
    pub audio: Bytes,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Jpod {
    pub audio: Bytes,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Nhk16 {
    pub audio: Bytes,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Shinmeikai8 {
    pub audio: Bytes,
    pub pitch_number: Option<u64>,
    pub pitch_pattern: Option<NonEmptyString>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Furigana {
    pub character: NonEmptyString,
    pub reading: NonEmptyString,
}
