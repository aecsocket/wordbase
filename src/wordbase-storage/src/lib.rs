#![doc = include_str!("../README.md")]
#![allow(missing_docs)]
#![allow(clippy::missing_errors_doc)]

pub mod archive;
pub mod backend;
pub mod codec;
pub mod import;
pub mod inbuilt;

pub use wordbase_api::*;
use {
    crate::import::ImportProgress,
    eyre::{Context, Result, eyre},
    futures::Stream,
    serde::{Deserialize, Serialize},
    std::path::Path,
    tokio::{fs, task::spawn_blocking},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DictionaryManifest {
    pub version: ManifestVersion,
    pub storage_backend: inbuilt::BackendKind,
    pub storage_codec: inbuilt::CodecKind,
    pub meta: DictionaryMeta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum ManifestVersion {
    V1,
}

#[derive(Debug)]
pub struct RecordRow {
    pub term_part: TermPart,
    pub record_id: RecordId,
    pub record: Record,
}

#[derive(Debug)]
pub struct Lookups<L = inbuilt::InbuiltLookups, C = inbuilt::InbuiltCodec> {
    lookups: L,
    codec: C,
}

impl<L: backend::Lookups, C: codec::Codec> Lookups<L, C> {
    pub fn new(backend: L, codec: C) -> Self {
        Self {
            lookups: backend,
            codec,
        }
    }

    pub fn lookup_lemma(&self, lemma: &str) -> Result<Vec<RecordRow>> {
        self.lookups.lookup_lemma(|| self.codec.decoder(), lemma)
    }
}

const MANIFEST_PATH: &str = "manifest.json";

pub enum ImportEvent {
    Progress(ImportProgress),
    Done,
}

pub fn import(data_dir: &Path) -> impl Stream<Item = Result<ImportEvent>> {
    async_stream::try_stream! {
        yield ImportEvent::Done;
    }
}

pub async fn open(data_dir: &Path) -> Result<(DictionaryManifest, Lookups)> {
    let manifest_path = data_dir.join(MANIFEST_PATH);
    let manifest = async {
        let file = fs::File::open(&manifest_path)
            .await
            .wrap_err("failed to open file")?;
        serde_json::from_reader::<_, DictionaryManifest>(file.into_std().await)
            .wrap_err("failed to parse manifest")
    }
    .await
    .wrap_err_with(|| eyre!("failed to read manifest file at {manifest_path:?}"))?;

    let data_dir = data_dir.to_path_buf();
    let lookups = spawn_blocking(move || inbuilt::open(&data_dir, manifest.storage_backend))
        .await?
        .wrap_err_with(|| {
            eyre!(
                "failed to open dictionary for lookups as {:?}",
                manifest.storage_backend
            )
        })?;
    let codec = inbuilt::codec(manifest.storage_codec)
        .wrap_err_with(|| eyre!("failed to create codec {:?}", manifest.storage_codec))?;
    Ok((manifest, Lookups { lookups, codec }))
}
