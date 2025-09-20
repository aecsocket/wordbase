#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

pub use wordbase_api::*;
use {
    eyre::{Context, Result, eyre},
    std::path::PathBuf,
    tokio::{fs, sync::RwLock},
    tracing::warn,
    uuid::Uuid,
    wordbase_storage::{
        archive::OpenArchive,
        backend::{Backend, RecordRow},
    },
};

type DefaultBackend = wordbase_storage::backend::Rocksdb;
type DefaultCodec = wordbase_storage::codec::Rkyv;
type Lookups = wordbase_storage::Lookups<<DefaultBackend as Backend>::Lookups, DefaultCodec>;

#[derive(Debug)]
pub struct Dictionaries {
    data_dir: PathBuf,
    open: Vec<OpenDictionary>,
}

#[derive(Debug)]
pub struct OpenDictionary {
    pub state: Dictionary,
    lookups: Lookups,
}

impl Dictionaries {
    pub async fn new(data_dir: impl Into<PathBuf>) -> Result<Self> {
        Self::new_(data_dir.into()).await
    }

    async fn new_(data_dir: PathBuf) -> Result<Self> {
        let mut dir = fs::read_dir(&data_dir)
            .await
            .wrap_err("failed to read data directory")?;
        let mut open = Vec::new();

        while let Some(entry) = dir
            .next_entry()
            .await
            .wrap_err("failed to read data directory entry")?
        {
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                warn!("Ignoring non-UTF-8 file {file_name:?}");
                continue;
            };

            async {
                let dict_id = Uuid::try_parse(file_name)
                    .map(DictionaryId)
                    .wrap_err("not a valid dictionary ID")?;

                let (manifest, lookups) = wordbase_storage::open::<DefaultBackend, DefaultCodec>(
                    &entry.path(),
                    DefaultCodec::default(),
                )
                .await
                .wrap_err("failed to open storage")?;

                open.push(OpenDictionary {
                    state: Dictionary {
                        id: dict_id,
                        meta: manifest.meta,
                        position: 0,
                    },
                    lookups,
                });
                eyre::Ok(())
            }
            .await
            .wrap_err_with(|| eyre!("failed to open dictionary '{file_name}'"))?;
        }

        Ok(Self { data_dir, open })
    }

    #[must_use]
    pub fn list(&self) -> &[OpenDictionary] {
        &self.open
    }

    fn dict_dir(&self, id: DictionaryId) -> PathBuf {
        self.data_dir.join(id.0.hyphenated().to_string())
    }

    pub async fn import(&mut self, open_archive: &dyn OpenArchive) -> Result<()> {
        let dict_id = DictionaryId::random();
        let dict_dir = self.dict_dir(dict_id);
        fs::create_dir_all(&dict_dir)
            .await
            .wrap_err_with(|| eyre!("failed to create dictionary directory {dict_dir:?}"))?;

        wordbase_storage::import::<DefaultBackend, DefaultCodec>(open_archive, &dict_dir).await?;
        let (manifest, lookups) = wordbase_storage::open::<DefaultBackend, DefaultCodec>(
            &dict_dir,
            DefaultCodec::default(),
        )
        .await
        .wrap_err("failed to reopen dictionary for lookups")?;

        self.open.push(OpenDictionary {
            state: Dictionary {
                id: dict_id,
                meta: manifest.meta,
                position: 0,
            },
            lookups,
        });
        Ok(())
    }

    pub async fn remove(&mut self, id: DictionaryId) -> Result<()> {
        self.open.retain(|dict| dict.state.id != id);
        let dict_dir = self.dict_dir(id);
        fs::remove_dir_all(&dict_dir)
            .await
            .wrap_err_with(|| eyre!("failed to remove dictionary directory {dict_dir:?}"))?;
        Ok(())
    }

    pub async fn lookup(&self, lemma: &str) -> Result<Vec<RecordRow>> {
        let rows = {
            let open = &self.open;
            open.iter()
                .map(|dict| dict.lookups.lookup(lemma))
                .collect::<Result<Vec<_>, _>>()?
        }
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
        Ok(rows)
    }
}
