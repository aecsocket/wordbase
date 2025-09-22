/// Texthooker sentence event received from a [TextractorSender] server, in the
/// [exSTATic] format.
///
/// [TextractorSender]: https://github.com/KamWithK/TextractorSender
/// [exSTATic]: https://github.com/KamWithK/exSTATic/
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct TexthookerSentence {
    /// Path of the process from which this texthooker sentence was extracted.
    ///
    /// This is not guaranteed to be in any format, but may be used as a
    /// persistent identifier.
    pub process_path: String,
    /// Extracted sentence.
    ///
    /// This may be malformed in some way, e.g. it may have trailing whitespace.
    pub sentence: String,
}
