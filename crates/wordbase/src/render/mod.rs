//! Allows rendering dictionary records out to a different format.

#[cfg(feature = "render-html")]
mod html;
#[cfg(feature = "render-html")]
pub use html::to_html;

use crate::RecordKind;

/// What [`RecordKind`]s are able to be rendered out.
pub const SUPPORTED_RECORD_KINDS: &[RecordKind] = &[
    // meta
    RecordKind::Frequency,
    RecordKind::JpnPitch,
    // glossaries
    RecordKind::GlossaryPlainText,
    RecordKind::GlossaryHtml,
    RecordKind::YomitanGlossary,
];
