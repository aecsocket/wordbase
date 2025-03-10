#![doc = include_str!("../README.md")]
#![expect(missing_docs)]
#![expect(clippy::missing_errors_doc)]

extern crate gtk4 as gtk;
extern crate libadwaita as adw;

mod dictionary;

pub use dictionary::*;
