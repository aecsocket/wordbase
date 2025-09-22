//! See [`Codec`].

use {eyre::Result, wordbase_types::Record};

/// Allows encoding a [`Record`] to bytes, and decoding bytes into a [`Record`].
pub trait Codec: Send + Sync + 'static {
    /// Type of [`Codec::encoder`].
    type Encoder: Encoder;
    /// Type of [`Codec::decoder`].
    type Decoder: Decoder;

    /// Creates an [`Encoder`].
    fn encoder(&self) -> Self::Encoder;

    /// Creates a [`Decoder`].
    fn decoder(&self) -> Self::Decoder;
}

/// Allows encoding a [`Record`] to bytes.
///
/// When encoding, try to re-use this as much as possible, as it may hold state
/// like allocations or a scratch space.
pub trait Encoder: Send + Sync + 'static {
    /// Type of [`Encoder::encode`].
    type Output<'enc>: AsRef<[u8]>;

    /// Encodes a [`Record`] to bytes.
    ///
    /// # Errors
    ///
    /// Implementation-specific.
    fn encode(&mut self, record: &Record) -> Result<Self::Output<'_>>;
}

/// Allows decoding bytes into a [`Record`].
///
/// When decoding, try to re-use this as much as possible, as it may hold state
/// like allocations or a scratch space.
pub trait Decoder: Send + Sync + 'static {
    /// Decodes bytes into a [`Record`].
    ///
    /// The bytes have no alignment guarantees.
    ///
    /// # Errors
    ///
    /// Implementation-specific.
    fn decode(&mut self, bytes: &[u8]) -> Result<Record>;
}
