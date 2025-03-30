use bytes::Bytes;
use serde::{Deserialize, Serialize};

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
    pub furigana: Vec<Furigana>,
    pub usage: Option<String>,
    pub category: Option<String>,
    pub pitch_accent: u32,
    pub silenced_mora: Vec<u32>,
    pub pronunciation: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Shinmeikai8 {
    pub audio: Bytes,
    pub pitch_number: u64,
    pub pitch_pattern: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Furigana {
    pub character: String,
    pub reading: String,
}
