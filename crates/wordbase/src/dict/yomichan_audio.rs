use {
    crate::NormString,
    bytes::Bytes,
    serde::{Deserialize, Serialize},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioFormat {
    Opus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    pub format: AudioFormat,
    pub data: Bytes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forvo {
    pub username: String,
    pub audio: Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jpod {
    pub audio: Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nhk16 {
    pub audio: Audio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shinmeikai8 {
    pub audio: Audio,
    pub pitch_number: Option<u64>,
    pub pitch_pattern: Option<NormString>,
}
