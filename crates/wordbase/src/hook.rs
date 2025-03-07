//! Protocol types for texthooking applications.
//!
//! A texthooker like [Textractor] is a program which reads the memory of
//! another program, extracts some text from its memory, and presents it to the
//! user. It is commonly used when playing [visual novels][vn], as the text
//! within a VN can usually not be directly copied and pasted. You can use a
//! texthooker in conjunction with an extension like [TextractorSender] to
//! open a WebSocket server which sends clients the extracted sentences.
//!
//! The Wordbase server is able to connect to a texthooker server, receive
//! [sentences], and forward them out to connected clients. Clients are also
//! able to connect to the Wordbase server and send out sentences, which are
//! then forwarded to all clients (including the sender). In this way, the
//! Wordbase server effectively acts as a broker between texthookers and
//! clients.
//!
//! [Textractor]: https://github.com/Artikash/Textractor/
//! [vn]: https://learnjapanese.moe/vn/
//! [TextractorSender]: https://github.com/KamWithK/TextractorSender
//! [sentences]: HookSentence

use serde::{Deserialize, Serialize};

/// Sentence which has been extracted from another process or application,
/// encoded as JSON.
///
/// See [`hook`].
///
/// [`hook`]: crate::hook
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookSentence {
    /// Process path which this sentence was extracted from.
    ///
    /// This can be used as a persistent identifier for which process sentences
    /// are being received from.
    pub process_path: String,
    /// Extracted sentence.
    ///
    /// This text has no guarantees other than being valid UTF-8 (which comes
    /// with the [`String`] type itself). It could have leading or trailing
    /// whitespace (including newlines), so it should be sanitized before being
    /// presented to the user.
    pub sentence: String,
}
