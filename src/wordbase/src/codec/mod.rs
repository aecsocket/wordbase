use {eyre::Result, wordbase_api::Record};

#[cfg(feature = "codec-rkyv")]
pub mod rkyv;
#[cfg(feature = "codec-rkyv")]
pub type Rkyv = rkyv::Codec;

#[cfg(feature = "codec-rmp")]
pub mod rmp;
#[cfg(feature = "codec-rmp")]
pub type Rmp = rmp::Codec;

pub trait Codec: Send + Sync + 'static {
    type Encoder: Encoder;
    type Decoder: Decoder;

    fn encoder() -> Self::Encoder;

    fn decoder() -> Self::Decoder;
}

pub trait Encoder: Send + Sync + 'static {
    fn encode(&mut self, record: &Record) -> Result<impl AsRef<[u8]>>;
}

pub trait Decoder: Send + Sync + 'static {
    fn decode(&mut self, bytes: &[u8]) -> Result<Record>;
}
