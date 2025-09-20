use {eyre::Result, std::fmt::Debug, wordbase_api::Record};

pub trait Codec: Send + Sync + Debug + 'static {
    type Encoder: Encoder;
    type Decoder: Decoder;

    fn encoder(&self) -> Self::Encoder;

    fn decoder(&self) -> Self::Decoder;
}

pub trait Encoder: Send + Sync + 'static {
    type Output<'enc>: AsRef<[u8]>;

    fn encode(&mut self, record: &Record) -> Result<Self::Output<'_>>;
}

pub trait Decoder: Send + Sync + 'static {
    fn decode(&mut self, bytes: &[u8]) -> Result<Record>;
}

#[cfg(feature = "codec-rkyv")]
pub mod rkyv;
#[cfg(feature = "codec-rkyv")]
pub type Rkyv = rkyv::Codec;

#[cfg(feature = "codec-rmp")]
pub mod rmp;
#[cfg(feature = "codec-rmp")]
pub type Rmp = rmp::Codec;
