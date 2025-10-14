#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

pub mod deinflect;
pub mod dictionaries;
pub mod error;
pub mod profiles;
pub mod storage;

pub use {
    error::{Error, Result},
    wordbase_core::*,
};
