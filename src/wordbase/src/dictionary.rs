use {
    crate::{
        import::{self, FinishImport, ImportProgress, OpenArchive},
        storage::Storage,
    },
    eyre::{Context, Result, eyre},
    futures::Stream,
    std::path::{Path, PathBuf},
    tokio::{fs, io::AsyncWriteExt, task::spawn_blocking},
    wordbase_api::DictionaryMeta,
};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DictionaryManifest {
    pub storage_strategy: StorageStrategy,
    pub meta: DictionaryMeta,
}

pub trait InbuiltStorage: Storage {
    fn strategy() -> StorageStrategy;
}

macro_rules! storage_strategies {
    ( $($storage_mod:ident + $codec_mod:ident => $name:ident),* ) => {
        $(impl InbuiltStorage for crate::storage::$storage_mod::Storage<crate::codec::$codec_mod::Codec> {
            fn strategy() -> StorageStrategy {
                StorageStrategy::$name
            }
        })*

        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub enum StorageStrategy {
            $($name,)*
        }

        impl StorageStrategy {
            pub fn open(&self, data_dir: &Path) -> Result<InbuiltLookups> {
                match self {
                    $(Self::$name => {
                        let storage = crate::storage::$storage_mod::Storage::new()
                            .wrap_err("failed to create storage")?;
                        let lookups = storage.open(data_dir)
                            .wrap_err("failed to open lookups")?;
                        Ok(InbuiltLookups::$name(lookups))
                    },)*
                }
            }
        }

        #[derive(Debug)]
        pub enum InbuiltLookups {
            $($name(crate::storage::$storage_mod::Lookups<crate::codec::$codec_mod::Codec>),)*
        }
    };
}

storage_strategies!(
    redb + rmp => RedbRmp,
    redb + rkyv => RedbRkyv,
    heed + rmp => HeedRmp,
    heed + rkyv => HeedRkyv,
    rocksdb + rmp => RocksDbRmp,
    rocksdb + rkyv => RocksDbRkyv
);

const MANIFEST_PATH: &str = "dictionary.json";

#[derive(Debug)]
pub enum ImportEvent {
    CreatedDir,
    CreatedStorage,
    ReadMeta(DictionaryMeta),
    WroteManifest,
    Progress(ImportProgress),
    Done,
}

pub fn import<S: InbuiltStorage>(
    storage: S,
    dictionary_path: PathBuf,
    open_archive: impl OpenArchive + 'static,
) -> impl Stream<Item = Result<ImportEvent>> {
    async_stream::try_stream! {
        fs::create_dir_all(&dictionary_path)
            .await
            .wrap_err_with(|| eyre!("failed to create {dictionary_path:?}"))?;
        let manifest_path = dictionary_path.join(MANIFEST_PATH);
        let mut manifest_file = fs::File::create(&manifest_path)
            .await
            .wrap_err_with(|| eyre!("failed to open manifest file at `{manifest_path:?}`"))?;
        yield ImportEvent::CreatedDir;

        let import_storage = spawn_blocking(move || storage.begin_import(&dictionary_path)).await??;
        yield ImportEvent::CreatedStorage;

        let (meta, import) = spawn_blocking(move || import::yomitan::start(open_archive)).await??;
        let manifest = DictionaryManifest {
            storage_strategy: S::strategy(),
            meta,
        };
        let manifest_data = serde_json::to_string_pretty(&manifest)
            .wrap_err("failed to serialize manifest")?;
        let meta = manifest.meta;
        yield ImportEvent::ReadMeta(meta);

        manifest_file.write_all(manifest_data.as_bytes())
            .await
            .wrap_err("failed to write manifest")?;
        yield ImportEvent::WroteManifest;

        let (tx_progress, rx_progress) = async_channel::bounded::<ImportProgress>(4);
        let import = spawn_blocking(move || import.finish(import_storage, tx_progress));
        while let Ok(progress) = rx_progress.recv().await {
            yield ImportEvent::Progress(progress);
        }

        import
            .await
            .wrap_err("failed to join import task")?
            .wrap_err("import failed")?;
        yield ImportEvent::Done;
    }
}

pub async fn open(dict_dir: &Path) -> Result<(DictionaryMeta, InbuiltLookups)> {
    let manifest_path = dict_dir.join(MANIFEST_PATH);
    let manifest = (async {
        let file = fs::File::open(&manifest_path)
            .await
            .wrap_err("failed to open file")?;
        let manifest = serde_json::from_reader::<_, DictionaryManifest>(file.into_std().await)
            .wrap_err("failed to deserialize")?;
        eyre::Ok(manifest)
    })
    .await
    .wrap_err_with(|| eyre!("failed to read manifest at {manifest_path:?}"))?;

    let lookups = manifest.storage_strategy.open(dict_dir)?;
    Ok((manifest.meta, lookups))
}
