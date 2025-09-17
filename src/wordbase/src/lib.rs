#![doc = include_str!("../README.md")]
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]

pub mod codec;
pub mod dictionary;
pub mod import;
pub mod storage;

pub use wordbase_api::*;
use {
    eyre::{Context, Result, eyre},
    std::path::Path,
    tokio::fs,
    tracing::trace,
    wordbase_api::uuid::Uuid,
};

pub struct Wordbase {}

const DICTIONARIES_PATH: &str = "dictionaries";

impl Wordbase {
    pub async fn new(data_path: impl AsRef<Path>) -> Result<Self> {
        Self::new_(data_path.as_ref()).await
    }

    async fn new_(data_path: &Path) -> Result<Self> {
        let dictionaries_dir = data_path.join(DICTIONARIES_PATH);
        fs::create_dir_all(&dictionaries_dir)
            .await
            .wrap_err_with(|| eyre!("failed to create {dictionaries_dir:?}"))?;

        let mut entries = fs::read_dir(&dictionaries_dir)
            .await
            .wrap_err("failed to list dictionaries")?;
        while let Some(entry) = entries
            .next_entry()
            .await
            .wrap_err("failed to list dictionary directory")?
        {
            let file_name = entry
                .file_name()
                .into_string()
                .map_err(|name| eyre!("non-UTF-8 file name {name:?}"))?;
            let Ok(dictionary_id) = Uuid::try_parse(&file_name).map(DictionaryId) else {
                trace!("Skipping {file_name:?} because it is not a valid UUID");
                continue;
            };
        }

        Self {}
    }
}
