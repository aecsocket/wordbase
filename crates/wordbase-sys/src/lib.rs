#![doc = include_str!("../README.md")]

// required for the generated dylib to link to wordbase,
// and for `uniffi-bindgen` to generate bindings for wordbase
extern crate wordbase;

/// Initialize Rust/Android integration.
#[cfg(feature = "android")]
pub fn android_init() {
    android_logger::init_once(
        android_logger::Config::default().with_max_level(tracing::log::LevelFilter::Trace),
    );
}
