#![doc = include_str!("../README.md")]

pub mod dictionary;
pub mod request;
pub mod response;

pub use {request::Request, response::Response};

/// Default port which a Wordbase server listens on.
pub const DEFAULT_PORT: u16 = 9518;
