#![doc = include_str!("../README.md")]

uniffi::setup_scaffolding!();

/// Foo bar docs
#[uniffi::export]
fn add(a: u32, b: u32) -> u32 {
    a + b
}

/// Foo bar baz docs
#[derive(uniffi::Object)]
pub struct Wordbase(wordbase_engine::Engine);

/// Foo bar baz new docs
#[uniffi::export]
pub async fn engine() -> Wordbase {
    Wordbase(wordbase_engine::Engine::new("").await.unwrap())
}

#[uniffi::export]
impl Wordbase {
    pub fn stuff(&self) -> Vec<wordbase::RecordKind> {
        wordbase::RecordKind::ALL.to_vec()
    }
}
