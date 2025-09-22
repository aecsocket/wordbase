//! [`wordbase_core::codec::Codec`] implementations.

pub use wordbase_core::codec::*;

#[cfg(feature = "codec-rkyv")]
pub mod rkyv;
#[cfg(feature = "codec-rkyv")]
pub use rkyv::Rkyv;

#[cfg(feature = "codec-rmp")]
pub mod rmp;
#[cfg(feature = "codec-rmp")]
pub use rmp::Rmp;
