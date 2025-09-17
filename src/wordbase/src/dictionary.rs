use {
    crate::{
        codec::{Codec, CodecKind},
        import::{self, FinishImport, ImportProgress, OpenArchive},
        storage::{self, Storage, StorageKind},
    },
    eyre::{Context, Result, eyre},
    futures::Stream,
    std::path::PathBuf,
    tokio::{fs, io::AsyncWriteExt, task::spawn_blocking},
    wordbase_api::DictionaryMeta,
};

#[derive(Debug)]
pub enum ImportEvent {
    CreatedDir,
    CreatedStorage,
    ReadMeta(DictionaryMeta),
    WroteManifest,
    Progress(ImportProgress),
    Done,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct DictionaryManifest {
    pub storage: StorageKind,
    pub codec: CodecKind,
    pub meta: DictionaryMeta,
}

const MANIFEST_PATH: &str = "dictionary.json";

pub fn import<S: Storage>(
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

        let mut import_storage = spawn_blocking(move || storage.begin_import(&dictionary_path)).await??;
        yield ImportEvent::CreatedStorage;

        let (meta, import) = spawn_blocking(move || import::yomitan::start(open_archive)).await??;
        let manifest = DictionaryManifest {
            storage: S::kind(),
            codec: S::Codec::kind(),
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
        let import = spawn_blocking(move || import.finish(&mut import_storage, tx_progress));
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

pub async fn open(dictionary_path: PathBuf) -> Result<()> {
    let manifest_path = dictionary_path.join(MANIFEST_PATH);
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

    let storage = match manifest.storage {
        StorageKind::Redb => storage::Redb::new(),
        StorageKind::Heed => storage::Heed::new(),
        StorageKind::RocksDb => storage::RocksDb::new().unwrap(), // TODO
    };

    Ok(())
}
