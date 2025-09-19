#![doc = include_str!("../README.md")]
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]

pub mod archive;
pub mod backend;
pub mod codec;
pub mod import;

pub use wordbase_api::*;
