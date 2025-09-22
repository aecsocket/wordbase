//! Logic for importing and opening dictionaries.

use {
    crate::{
        archive::OpenArchive,
        codec::Codec,
        importer::{self, ImportProgress, Importer},
        storage::{ImportStorage, ImportTransaction, LookupStorage, RecordRow, Storage},
    },
    eyre::{Context, Result, bail, eyre},
    rayon::prelude::*,
    std::{cmp, path::Path},
    tokio::fs,
    tracing::debug,
    wordbase_types::DictionaryMeta,
};

const MANIFEST_PATH: &str = "manifest.json";
const MANIFEST_VERSION: u64 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DictionaryManifest {
    pub version: u64,
    pub meta: DictionaryMeta,
}

pub enum ImportEvent {
    Progress(ImportProgress),
    Done,
}

pub async fn import<S: Storage, C: Codec + Default>(
    open_archive: &dyn OpenArchive,
    dict_dir: &Path,
    importers: &[&dyn Importer],
) -> Result<()> {
    let import_ops = importers
        .par_iter()
        .filter_map(|&importer| match importer.start(open_archive) {
            Ok((meta, finish)) => Some((importer, meta, finish)),
            Err(err) => {
                debug!("Archive could not be imported by {importer:?}: {err:#}");
                None
            }
        })
        .collect::<Vec<_>>();

    let (importer, meta, finish) = match <[_; 1]>::try_from(import_ops) {
        Ok([x]) => x,
        Err(v) if v.is_empty() => bail!("no importers for archive"),
        Err(v) => bail!(
            "multiple importers for archive: {:?}",
            v.into_iter()
                .map(|(importer, _, _)| importer)
                .collect::<Vec<_>>()
        ),
    };
    debug!("Using importer {importer:?} for {meta:?}");

    let codec = C::default();
    let mut storage = S::create_import_storage(dict_dir)
        .await
        .wrap_err("failed to create import storage")?;
    let storage_txn = storage
        .transaction()
        .wrap_err("failed to begin transaction")?;

    {
        let import_txn = importer::transaction(&storage_txn, &codec);
        let (tx_progress, rx_progress) = async_channel::bounded(4);
        finish.finish(&import_txn, tx_progress)?;
    }

    storage_txn
        .commit()
        .wrap_err("failed to commit transaction")?;
    storage.commit().wrap_err("failed to commit storage")?;

    let manifest_path = dict_dir.join(MANIFEST_PATH);
    async {
        let manifest = serde_json::to_string_pretty(&DictionaryManifest {
            version: MANIFEST_VERSION,
            meta,
        })
        .wrap_err("failed to serialize manifest")?;
        fs::write(&manifest_path, &manifest)
            .await
            .wrap_err("failed to write to file")?;
        eyre::Ok(())
    }
    .await
    .wrap_err_with(|| eyre!("failed to write manifest to {manifest_path:?}"))?;

    Ok(())
}

#[derive(Debug)]
pub struct Lookups<S, C> {
    storage: S,
    codec: C,
}

impl<S: LookupStorage, C: Codec> Lookups<S, C> {
    pub fn lookup(&self, lemma: &str) -> Result<Vec<RecordRow>> {
        self.storage.lookup(|| self.codec.decoder(), lemma)
    }
}

pub async fn open<S: Storage, C: Codec>(
    dict_dir: &Path,
    codec: C,
) -> Result<(DictionaryManifest, Lookups<S::Lookups, C>)> {
    let manifest_path = dict_dir.join(MANIFEST_PATH);
    let manifest = async {
        let file = fs::File::open(&manifest_path)
            .await
            .wrap_err("failed to open file")?;
        serde_json::from_reader::<_, DictionaryManifest>(file.into_std().await)
            .wrap_err("failed to parse manifest")
    }
    .await
    .wrap_err_with(|| eyre!("failed to read manifest file at {manifest_path:?}"))?;

    let version = manifest.version;
    match version.cmp(&MANIFEST_VERSION) {
        cmp::Ordering::Equal => {}
        cmp::Ordering::Greater => {
            bail!("dictionary version {version} is newer than our version {MANIFEST_VERSION}");
        }
        cmp::Ordering::Less => {
            bail!("dictionary version {version} is older than our version {MANIFEST_VERSION}");
        }
    }

    let dict_dir = dict_dir.to_path_buf();
    let lookups = S::open(&dict_dir).await?;
    Ok((
        manifest,
        Lookups {
            storage: lookups,
            codec,
        },
    ))
}
