#![doc = include_str!("../README.md")]

uniffi::setup_scaffolding!();

#[uniffi::export]
fn add(a: u32, b: u32) -> u32 {
    a + b
}
