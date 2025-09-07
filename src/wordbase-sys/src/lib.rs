#![doc = include_str!("../README.md")]

// required for the generated dylib to link to wordbase,
// and for `uniffi-bindgen` to generate bindings for wordbase
extern crate wordbase;
