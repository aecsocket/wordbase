use anyhow::{Context as _, Result};
use lindera::dictionary::DictionaryKind;

use crate::Engine;

#[derive(Debug, Clone)]
pub struct Deinflector;

impl Deinflector {
    pub fn new() -> Result<Self> {
        let dictionary = lindera::dictionary::load_dictionary_from_kind(DictionaryKind::UniDic)
            .context("foo")?;
    }
}
