use anyhow::{Context, Result, bail};
use async_compression::futures::bufread::XzDecoder;
use bytes::Bytes;
use derive_more::Deref;
use foldhash::{HashMap, HashMapExt};
use futures::{AsyncRead, AsyncReadExt, StreamExt, future::BoxFuture, io::Cursor};
use serde::{Deserialize, de::DeserializeOwned};
use sqlx::{Sqlite, Transaction};
use tokio::sync::mpsc;
use wordbase::{
    DictionaryId, DictionaryKind, DictionaryMeta, RecordType, Term,
    dict::yomichan_audio::{Forvo, Jpod, Nhk16, Shinmeikai8},
};

use crate::{CHANNEL_BUF_CAP, Engine, import::insert_dictionary};

use super::{ImportContinue, ImportKind, ImportTracker, insert_term};

pub struct YomichanAudio;

impl ImportKind for YomichanAudio {
    fn is_of_kind(&self, archive: Bytes) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move { validate(&archive).await })
    }

    fn start_import<'a>(
        &'a self,
        engine: &'a Engine,
        archive: Bytes,
    ) -> BoxFuture<'a, Result<(ImportTracker, ImportContinue<'a>)>> {
        Box::pin(async move {
            let mut meta = DictionaryMeta::new(
                DictionaryKind::YomichanAudio,
                "Yomichan Japanese Local Audio",
                "2023-06-11-opus",
            );
            meta.url = Some("https://github.com/yomidevs/local-audio-yomichan".into());
            let (send_progress, recv_progress) = mpsc::channel(CHANNEL_BUF_CAP);
            Ok((
                ImportTracker {
                    meta: meta.clone(),
                    recv_progress,
                },
                Box::pin(import(engine, archive, meta, send_progress)) as ImportContinue,
            ))
        })
    }
}

const FORVO_PATH: &str = "user_files/forvo_files/";
const JPOD_INDEX: &str = "user_files/jpod_files/index.json";
const JPOD_MEDIA: &str = "user_files/jpod_files/media/";
const NHK16_ENTRIES: &str = "user_files/nhk16_files/entries.json";
const NHK16_AUDIO: &str = "user_files/nhk16_files/audio/";
const SHINMEIKAI8_INDEX: &str = "user_files/shinmeikai8_files/index.json";
const SHINMEIKAI8_MEDIA: &str = "user_files/shinmeikai8_files/media";

const MARKER_PATHS: &[&str] = &[
    FORVO_PATH,
    JPOD_INDEX,
    JPOD_MEDIA,
    NHK16_ENTRIES,
    NHK16_AUDIO,
    SHINMEIKAI8_INDEX,
    SHINMEIKAI8_MEDIA,
];

async fn validate(archive: &[u8]) -> Result<()> {
    let mut entries = async_tar::Archive::new(XzDecoder::new(Cursor::new(archive)))
        .entries()
        .context("failed to read archive entries")?;
    while let Some(entry) = entries.next().await {
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(path) = entry.path() else {
            continue;
        };
        let Some(path) = path.to_str() else {
            continue;
        };
        if MARKER_PATHS.contains(&path) {
            return Ok(());
        }
    }
    bail!("missing one of {MARKER_PATHS:?}");
}

async fn import(
    engine: &Engine,
    archive: Bytes,
    meta: DictionaryMeta,
    send_progress: mpsc::Sender<f64>,
) -> Result<()> {
    let mut tx = engine
        .db
        .begin()
        .await
        .context("failed to begin transaction")?;
    let source = insert_dictionary(&mut tx, &meta)
        .await
        .context("failed to insert dictionary")?;

    let mut entries = async_tar::Archive::new(XzDecoder::new(Cursor::new(&archive)))
        .entries()
        .context("failed to read archive entries")?;
    let mut scratch = Vec::new();
    let mut jpod_index = None;
    let mut nhk16_index = None;
    let mut shinmeikai8_index = None;
    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("failed to read archive entry")?;
        let path = entry.path().context("failed to read entry file path")?;
        let path = path
            .to_str()
            .with_context(|| format!("path {path:?} is not UTF-8"))?
            .to_owned();

        (async {
            if let Some(path) = path.strip_prefix(FORVO_PATH) {
                import_forvo(&mut tx, source, &mut scratch, path, &mut entry)
                    .await
                    .context("failed to import Forvo file")?;
            } else if path == JPOD_INDEX {
                jpod_index = Some(
                    parse_index::<JpodIndex, _>(&mut entry)
                        .await
                        .context("failed to parse JPod index")?,
                );
            } else if let Some(path) = path.strip_prefix(JPOD_MEDIA) {
                import_by_index(
                    &mut tx,
                    source,
                    &mut scratch,
                    path,
                    &mut entry,
                    jpod_index.as_ref(),
                    |audio| Jpod { audio },
                )
                .await
                .context("failed to import JPod file")?;
            } else if path == NHK16_ENTRIES {
                nhk16_index = Some(
                    parse_index::<Nhk16Index, _>(&mut entry)
                        .await
                        .context("failed to parse NHK index")?,
                );
            } else if let Some(path) = path.strip_prefix(NHK16_AUDIO) {
                import_by_index(
                    &mut tx,
                    source,
                    &mut scratch,
                    path,
                    &mut entry,
                    nhk16_index.as_ref(),
                    |audio| Nhk16 { audio },
                )
                .await
                .context("failed to import NHK file")?;
            } else if path == SHINMEIKAI8_INDEX {
                shinmeikai8_index = Some(
                    parse_index::<Shinmeikai8Index, _>(&mut entry)
                        .await
                        .context("failed to parse Shinmeikai index")?,
                )
            } else if let Some(path) = path.strip_prefix(SHINMEIKAI8_MEDIA) {
                import_by_index(
                    &mut tx,
                    source,
                    &mut scratch,
                    path,
                    &mut entry,
                    shinmeikai8_index.as_ref(),
                    |audio| Shinmeikai8 { audio },
                )
                .await
                .context("failed to import Shinmeikai file")?;
            }
            anyhow::Ok(())
        })
        .await
        .with_context(|| format!("failed to process `{path}`"))?;
    }

    todo!()
}

#[derive(Debug, Deref)]
struct Index {
    path_to_term: HashMap<String, Term>,
}

async fn parse_index<I: Into<Index> + DeserializeOwned, R: AsyncRead + Unpin>(
    entry: &mut async_tar::Entry<R>,
) -> Result<Index> {
    let mut buf = Vec::new();
    entry
        .read_to_end(&mut buf)
        .await
        .context("failed to read file into memory")?;
    let raw_index = serde_json::from_reader::<_, I>(std::io::Cursor::new(buf))
        .context("failed to parse index")?;
    Ok(raw_index.into())
}

async fn import_forvo<R: AsyncRead + Unpin>(
    tx: &mut Transaction<'_, Sqlite>,
    source: DictionaryId,
    scratch: &mut Vec<u8>,
    path: &str,
    entry: &mut async_tar::Entry<R>,
) -> Result<()> {
    let mut parts = path.split('/');
    let username = parts
        .next()
        .map(ToOwned::to_owned)
        .context("no Forvo username in path")?;
    let headword_path = parts.next().context("no headword in path")?;
    let headword = headword_path
        .rsplit_once('.')
        .map_or(headword_path, |(name, _)| name);

    let mut audio = Vec::new();
    entry
        .read_to_end(&mut audio)
        .await
        .context("failed to read file into memory")?;

    insert_term(
        tx,
        source,
        &Term::new(headword),
        &Forvo {
            username,
            audio: Bytes::from(audio),
        },
        scratch,
    )
    .await
    .context("failed to insert term")?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct JpodIndex {
    headwords: HashMap<String, Vec<String>>,
}

impl From<JpodIndex> for Index {
    fn from(value: JpodIndex) -> Self {
        let mut path_to_term = HashMap::new();
        for (headword, paths) in value.headwords {
            for path in paths {
                path_to_term.insert(path, Term::new(&headword));
            }
        }
        Self { path_to_term }
    }
}

async fn import_by_index<R: AsyncRead + Unpin, T: RecordType>(
    tx: &mut Transaction<'_, Sqlite>,
    source: DictionaryId,
    scratch: &mut Vec<u8>,
    path: &str,
    entry: &mut async_tar::Entry<R>,
    index: Option<&Index>,
    to_record: impl FnOnce(Bytes) -> T,
) -> Result<()> {
    let index = index.context("index has not been parsed yet")?;
    let term = index
        .path_to_term
        .get(path)
        .with_context(|| format!("index does not contain an entry for `{path}`"))?;

    let mut audio = Vec::new();
    entry
        .read_to_end(&mut audio)
        .await
        .context("failed to read file into memory")?;

    insert_term(tx, source, term, &to_record(Bytes::from(audio)), scratch)
        .await
        .context("failed to insert term")?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct Nhk16Index {}

impl From<Nhk16Index> for Index {
    fn from(value: Nhk16Index) -> Self {
        todo!();
    }
}

#[derive(Debug, Deserialize)]
struct Shinmeikai8Index {}

impl From<Shinmeikai8Index> for Index {
    fn from(value: Shinmeikai8Index) -> Self {
        todo!();
    }
}
