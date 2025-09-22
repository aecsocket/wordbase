use {
    crate::storage::EngineStorage,
    arc_swap::ArcSwap,
    derive_more::{Deref, DerefMut},
    eyre::{Context, ContextCompat, Result, eyre},
    foldhash::HashSet,
    serde::Serialize,
    std::{path::PathBuf, sync::Arc},
    tokio::fs,
    tracing::warn,
    uuid::Uuid,
    wordbase_core::{archive::OpenArchive, dictionary, importer::Importer, storage::Storage},
    wordbase_core_storage::{codec, storage},
    wordbase_types::{DictionaryId, DictionaryMeta, Record, RecordId, Term},
};

type DefaultStorage = storage::Rocksdb;
type DefaultCodec = codec::Rkyv;
type Lookups = dictionary::Lookups<<DefaultStorage as Storage>::Lookups, DefaultCodec>;

const IMPORTERS: &[&dyn Importer] = wordbase_core_import::IMPORTERS;

#[derive(Debug)]
pub struct Dictionaries {
    data_dir: PathBuf,
    open: ArcSwap<OpenDictionaries>,
}

#[derive(Debug, Clone, Deref, DerefMut, Serialize)]
pub struct OpenDictionaries(pub Vec<Arc<OpenDictionary>>);

#[derive(Debug, Serialize)]
pub struct OpenDictionary {
    pub id: DictionaryId,
    pub meta: DictionaryMeta,
    #[serde(skip_serializing)]
    pub lookups: Lookups,
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

                let (manifest, lookups) = dictionary::open::<DefaultStorage, DefaultCodec>(
                    &entry.path(),
                    DefaultCodec::default(),
                )
                .await
                .wrap_err("failed to open storage")?;

                open.push(Arc::new(OpenDictionary {
                    id: dict_id,
                    meta: manifest.meta,
                    lookups,
                }));
                eyre::Ok(())
            }
            .await;
            if let Err(err) = result {
                warn!("Failed to open dictionary {file_name:?}: {err:#}");
            }
        }

        Ok(Self {
            data_dir,
            open: ArcSwap::from_pointee(OpenDictionaries(open)),
        })
    }

    pub fn all(&self) -> Arc<OpenDictionaries> {
        self.open.load().clone()
    }

    fn dict_dir(&self, id: DictionaryId) -> PathBuf {
        self.data_dir.join(id.0.hyphenated().to_string())
    }

    pub async fn import(&self, open_archive: &dyn OpenArchive) -> Result<DictionaryId> {
        let dict_id = DictionaryId::random();
        let dict_dir = self.dict_dir(dict_id);
        fs::create_dir_all(&dict_dir)
            .await
            .wrap_err_with(|| eyre!("failed to create dictionary directory {dict_dir:?}"))?;

        dictionary::import::<DefaultStorage, DefaultCodec>(open_archive, &dict_dir, IMPORTERS)
            .await?;
        let (manifest, lookups) =
            dictionary::open::<DefaultStorage, DefaultCodec>(&dict_dir, DefaultCodec::default())
                .await
                .wrap_err("failed to reopen dictionary for lookups")?;

        let mut dicts = OpenDictionaries::clone(&self.open.load());
        dicts.push(Arc::new(OpenDictionary {
            id: dict_id,
            meta: manifest.meta,
            lookups,
        }));
        self.open.store(Arc::new(dicts));

        Ok(dict_id)
    }

    pub async fn remove(&self, dict_id: DictionaryId) -> Result<()> {
        let mut dicts = OpenDictionaries::clone(&self.open.load());
        dicts.retain(|dict| dict.id != dict_id);
        self.open.store(Arc::new(dicts));

        let dict_dir = self.dict_dir(dict_id);
        fs::remove_dir_all(&dict_dir)
            .await
            .wrap_err_with(|| eyre!("failed to remove dictionary directory {dict_dir:?}"))?;
        Ok(())
    }

    // sorting deferred to caller
    pub fn lookup(
        &self,
        lemma: &str,
        enabled_dicts: &HashSet<DictionaryId>,
    ) -> Result<Vec<RecordEntry>> {
        self.open
            .load()
            .iter()
            .filter(|dict| enabled_dicts.contains(&dict.id))
            .flat_map(|dict| match dict.lookups.lookup(lemma) {
                Ok(x) => x
                    .into_iter()
                    .map(|row| Ok((dict.clone(), row)))
                    .collect::<Vec<_>>(),
                Err(err) => vec![Err(err).wrap_err_with(|| {
                    eyre!(
                        "failed to look up in dictionary {:?} ({:?})",
                        dict.meta.name,
                        dict.id
                    )
                })],
            })
            .map(|r| {
                let (dictionary, row) = r?;
                Ok(RecordEntry {
                    dictionary,
                    term: row.term,
                    record_id: row.record_id,
                    record: row.record,
                })
            })
            .collect::<Result<Vec<_>, _>>()
    }
}

#[derive(Debug)]
pub struct RecordEntry {
    pub dictionary: Arc<OpenDictionary>,
    pub term: Term,
    pub record_id: RecordId,
    pub record: Record,
}

impl EngineStorage {
    pub fn dictionaries(&self) -> Arc<OpenDictionaries> {
        self.dictionaries.all()
    }

    pub async fn import_dictionary(&self, open_archive: &dyn OpenArchive) -> Result<DictionaryId> {
        self.dictionaries.import(open_archive).await
    }
}
