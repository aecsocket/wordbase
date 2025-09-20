#[cfg(feature = "codec-rkyv")]
pub mod rkyv;
#[cfg(feature = "codec-rkyv")]
pub type Rkyv = rkyv::Codec;

#[cfg(feature = "codec-rmp")]
pub mod rmp;
#[cfg(feature = "codec-rmp")]
pub type Rmp = rmp::Codec;
