//! [Local Audio Server for Yomichan][las] audio collection format.
//!
//! [las]: https://github.com/yomidevs/local-audio-yomichan
// TODO: I'll be honest I have no clue where these audio sources actually come from.
// Docs are my best guess.

use {
    crate::NormString,
    bytes::Bytes,
    serde::{Deserialize, Serialize},
};

/// What file type [`Audio::data`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioFormat {
    /// Opus audio format.
    Opus,
}

/// Audio file data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Audio {
    /// File type of [`Audio::data`].
    pub format: AudioFormat,
    /// Raw audio file data.
    pub data: Bytes,
}

/// [Forvo] audio.
///
/// [Forvo]: https://forvo.com/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Forvo {
    /// Username of the speaker.
    pub username: String,
    /// Audio data.
    pub audio: Audio,
}

/// [JapanesePod101][jpod] audio.
///
/// [jpod]: https://www.japanesepod101.com/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jpod {
    /// Audio data.
    pub audio: Audio,
}

/// [NHK] audio.
///
/// [NHK]: https://www.nhk.or.jp/
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nhk16 {
    /// Audio data.
    pub audio: Audio,
}

/// [Shin Meikai] version 8 audio.
///
/// [Shin Meikai]: https://en.wikipedia.org/wiki/Shin_Meikai_kokugo_jiten
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shinmeikai8 {
    /// Audio data.
    pub audio: Audio,
    /// Pitch position of this pronunciation.
    ///
    /// See [`yomitan::Pitch::position`].
    ///
    /// [`yomitan::Pitch::position`]: crate::dict::yomitan::Pitch::position
    pub pitch_number: Option<u64>,
    /// Pitch pattern of this pronunciation.
    ///
    /// The downstep is indicated by a `＼`, for example `読＼む`.
    pub pitch_pattern: Option<NormString>,
}
