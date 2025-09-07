//! [Local Audio Server for Yomichan][las] audio collection format.
//!
//! [las]: https://github.com/yomidevs/local-audio-yomichan
// TODO: I'll be honest I have no clue where these audio sources actually come
// from. Docs are my best guess.

use {
    super::jpn::PitchPosition,
    crate::NormString,
    bytes::Bytes,
    derive_more::Display,
    serde::{Deserialize, Serialize},
};

/// What file type [`Audio::data`] is.
#[derive(Debug, Display, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum AudioFormat {
    /// Opus audio format.
    #[display("ogg")]
    Opus,
    /// MP3 audio format.
    #[display("mp3")]
    Mp3,
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
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
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Jpod {
    /// Audio data.
    pub audio: Audio,
}

/// [NHK] audio.
///
/// [NHK]: https://www.nhk.or.jp/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Nhk16 {
    /// Audio data.
    pub audio: Audio,
    /// Pitch positions of this pronunciation.
    ///
    /// See [`yomitan::Pitch::position`].
    ///
    /// [`yomitan::Pitch::position`]: crate::dict::yomitan::Pitch::position
    pub pitch_positions: Vec<PitchPosition>,
}

/// [Shin Meikai] version 8 audio.
///
/// [Shin Meikai]: https://en.wikipedia.org/wiki/Shin_Meikai_kokugo_jiten
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct Shinmeikai8 {
    /// Audio data.
    pub audio: Audio,
    /// Pitch position of this pronunciation.
    ///
    /// See [`yomitan::Pitch::position`].
    ///
    /// [`yomitan::Pitch::position`]: crate::dict::yomitan::Pitch::position
    pub pitch_number: Option<PitchPosition>,
    /// Pitch pattern of this pronunciation.
    ///
    /// The downstep is indicated by a `＼`, for example `読＼む`.
    pub pitch_pattern: Option<NormString>,
}

#[cfg(feature = "uniffi")]
const _: () = {
    use data_encoding::BASE64;

    #[derive(uniffi::Record)]
    pub struct AudioFfi {
        pub format: AudioFormat,
        // TODO: if we use `Vec<u8>`, in Kotlin this maps to a `ByteArray`.
        // `ByteArray.equals` does not check actual contents, just reference equality.
        // This breaks Android Compose, because when rendering records,
        // and there is an audio record in the list, Compose will always think
        // that the records have been modified, causing a recomposition.
        //
        // See <https://github.com/mozilla/uniffi-rs/issues/1985>.
        //
        // To work around this, we (de/en)code this data as base 64.
        // Theoretically we could do what the above issue proposes and wrap `Vec<u8>`,
        // but base 64 is simpler.
        pub data: String,
    }

    uniffi::custom_type!(Audio, AudioFfi, {
        lower: |x| AudioFfi {
            format: x.format,
            data: BASE64.encode(&x.data),
        },
        try_lift: |x| Ok(Audio {
            format: x.format,
            data: Bytes::from(BASE64.decode(x.data.as_bytes())?),
        })
    });
};
