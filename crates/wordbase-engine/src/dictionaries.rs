use {
    eyre::{Context, ContextCompat, Result, eyre},
    foldhash::HashSet,
    itertools::Itertools,
    std::path::PathBuf,
    tokio::{
        fs,
        sync::{RwLock, RwLockReadGuard},
    },
    tracing::warn,
    uuid::Uuid,
    wordbase_api::{Dictionary, DictionaryId, Record, RecordId},
    wordbase_storage::{archive::OpenArchive, backend::Backend},
};

type DefaultBackend = wordbase_storage::backend::Rocksdb;
type DefaultCodec = wordbase_storage::codec::Rkyv;
type Lookups = wordbase_storage::Lookups<<DefaultBackend as Backend>::Lookups, DefaultCodec>;

#[derive(Debug)]
pub struct Dictionaries {
    data_dir: PathBuf,
    open: RwLock<Vec<OpenDictionary>>,
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

            let result = async {
                let file_name = file_name.to_str().wrap_err("file name is not UTF-8")?;

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
            .await;
            if let Err(err) = result {
                warn!("Failed to open dictionary {file_name:?}: {err:#}");
            }
        }

        Ok(Self {
            data_dir,
            open: RwLock::new(open),
        })
    }

    pub async fn list(&self) -> RwLockReadGuard<'_, Vec<OpenDictionary>> {
        self.open.read().await
    }

    fn dict_dir(&self, id: DictionaryId) -> PathBuf {
        self.data_dir.join(id.0.hyphenated().to_string())
    }

    pub async fn import(&self, open_archive: &dyn OpenArchive) -> Result<()> {
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

        self.open.write().await.push(OpenDictionary {
            state: Dictionary {
                id: dict_id,
                meta: manifest.meta,
                position: 0,
            },
            lookups,
        });
        Ok(())
    }

    pub async fn remove(&self, id: DictionaryId) -> Result<()> {
        self.open.write().await.retain(|dict| dict.state.id != id);
        let dict_dir = self.dict_dir(id);
        fs::remove_dir_all(&dict_dir)
            .await
            .wrap_err_with(|| eyre!("failed to remove dictionary directory {dict_dir:?}"))?;
        Ok(())
    }

    pub async fn lookup(
        &self,
        lemma: &str,
        enabled_dicts: &HashSet<DictionaryId>,
    ) -> Result<Vec<RecordEntry>> {
        let mut entries = {
            let open = self.open.read().await;
            open.iter()
                .filter(|dict| enabled_dicts.contains(&dict.state.id))
                .map(|dict| {
                    dict.lookups.lookup(lemma).map(|rows| {
                        rows.into_iter().map(|row| RecordEntry {
                            dictionary_id: dict.state.id,
                            dictionary_position: dict.state.position,
                            term_part: row.term_part,
                            record_id: row.record_id,
                            record: row.record,
                        })
                    })
                })
                .flatten_ok()
                .collect::<Result<Vec<_>, _>>()?
        };

        // TODO more sorting
        entries.sort_unstable_by_key(|entry| entry.dictionary_position);

        Ok(entries)
    }
}

#[derive(Debug)]
pub struct RecordEntry {
    pub dictionary_id: DictionaryId,
    pub dictionary_position: i64,
    pub term_part: TermPart,
    pub record_id: RecordId,
    pub record: Record,
}
