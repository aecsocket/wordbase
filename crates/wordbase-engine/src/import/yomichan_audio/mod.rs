use anyhow::{Context, Result, bail};
use async_compression::futures::bufread::XzDecoder;
use async_tar::EntryType;
use bytes::Bytes;
use derive_more::Deref;
use foldhash::{HashMap, HashMapExt};
use futures::{AsyncRead, AsyncReadExt, StreamExt, future::BoxFuture, io::Cursor};
use serde::{Deserialize, de::DeserializeOwned};
use sqlx::{Sqlite, Transaction};
use tokio::sync::mpsc;
use tracing::{debug, trace};
use wordbase::{
    DictionaryId, DictionaryKind, DictionaryMeta, RecordType, Term,
    dict::yomichan_audio::{Forvo, Jpod, Nhk16, Shinmeikai8},
};

use crate::{CHANNEL_BUF_CAP, Engine, import::insert_dictionary};

use super::{ImportContinue, ImportKind, ImportStarted, insert_term};

pub struct YomichanAudio;

impl ImportKind for YomichanAudio {
    fn is_of_kind(&self, archive: Bytes) -> BoxFuture<'_, Result<()>> {
        Box::pin(async move { validate(&archive).await })
    }

    fn start_import<'a>(
        &'a self,
        engine: &'a Engine,
        archive: Bytes,
    ) -> BoxFuture<'a, Result<(ImportStarted, ImportContinue<'a>)>> {
        Box::pin(async move {
            let mut meta = DictionaryMeta::new(
                DictionaryKind::YomichanAudio,
                "Yomichan Japanese Local Audio",
                "2023-06-11-opus",
            );
            meta.url = Some("https://github.com/yomidevs/local-audio-yomichan".into());
            let (send_progress, recv_progress) = mpsc::channel(CHANNEL_BUF_CAP);
            Ok((
                ImportStarted {
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
const NHK16_INDEX: &str = "user_files/nhk16_files/entries.json";
const NHK16_AUDIO: &str = "user_files/nhk16_files/audio/";
const SHINMEIKAI8_INDEX: &str = "user_files/shinmeikai8_files/index.json";
const SHINMEIKAI8_MEDIA: &str = "user_files/shinmeikai8_files/media";

const MARKER_PATHS: &[&str] = &[
    FORVO_PATH,
    JPOD_INDEX,
    JPOD_MEDIA,
    NHK16_INDEX,
    NHK16_AUDIO,
    SHINMEIKAI8_INDEX,
    SHINMEIKAI8_MEDIA,
];

async fn validate(archive: &[u8]) -> Result<()> {
    let archive = async_tar::Archive::new(XzDecoder::new(Cursor::new(archive)));
    let mut entries = archive
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
) -> Result<DictionaryId> {
    let mut tx = engine
        .db
        .begin()
        .await
        .context("failed to begin transaction")?;
    let dictionary_id = insert_dictionary(&mut tx, &meta)
        .await
        .context("failed to insert dictionary")?;

    debug!("Counting entries and parsing indexes");
    let mut jpod_index = None;
    let mut nhk16_index = None;
    let mut shinmeikai8_index = None;
    let mut entries = async_tar::Archive::new(XzDecoder::new(Cursor::new(&archive)))
        .entries()
        .context("failed to read archive entries")?;
    let mut num_entries = 0usize;
    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("failed to read archive entry")?;
        let path = entry.path().context("failed to read entry file path")?;
        let path = path
            .to_str()
            .with_context(|| format!("path {path:?} is not UTF-8"))?
            .to_owned();

        (async {
            match path.as_str() {
                JPOD_INDEX => {
                    jpod_index = Some(parse_index::<GenericIndex, _>(&mut entry).await?);
                }
                NHK16_INDEX => {
                    nhk16_index = Some(parse_index::<Nhk16Index, _>(&mut entry).await?);
                }
                SHINMEIKAI8_INDEX => {
                    shinmeikai8_index = Some(parse_index::<GenericIndex, _>(&mut entry).await?);
                }
                _ => {}
            }
            anyhow::Ok(())
        })
        .await
        .with_context(|| format!("failed to process `{path}`"))?;

        num_entries += 1;
        if num_entries % 10_000 == 0 {
            debug!("{num_entries} entries...");
        }
    }
    debug!("{num_entries} total entries");

    let jpod_index = jpod_index.with_context(|| format!("no JPod index at `{JPOD_INDEX}`"))?;
    let nhk16_index = nhk16_index.with_context(|| format!("no NHK index at `{NHK16_INDEX}`"))?;
    let shinmeikai8_index = shinmeikai8_index
        .with_context(|| format!("no Shinmeikai index at `{SHINMEIKAI8_INDEX}`"))?;

    let mut entries = async_tar::Archive::new(XzDecoder::new(Cursor::new(&archive)))
        .entries()
        .context("failed to read archive entries")?;
    let mut entries_done = 0usize;
    let mut scratch = Vec::new();
    while let Some(entry) = entries.next().await {
        let mut entry = entry.context("failed to read archive entry")?;
        if entry.header().entry_type() != EntryType::Regular {
            continue;
        }

        let path = entry.path().context("failed to read entry file path")?;
        let path = path
            .to_str()
            .with_context(|| format!("path {path:?} is not UTF-8"))?
            .to_owned();

        (async {
            if let Some(path) = path.strip_prefix(FORVO_PATH) {
                import_forvo(&mut tx, dictionary_id, &mut scratch, path, &mut entry)
                    .await
                    .context("failed to import Forvo file")?;
            } else if let Some(path) = path.strip_prefix(JPOD_MEDIA) {
                import_by_index(
                    &mut tx,
                    dictionary_id,
                    &mut scratch,
                    path,
                    &mut entry,
                    &jpod_index,
                    |audio| Jpod { audio },
                )
                .await?;
            } else if let Some(path) = path.strip_prefix(NHK16_AUDIO) {
                import_by_index(
                    &mut tx,
                    dictionary_id,
                    &mut scratch,
                    path,
                    &mut entry,
                    &nhk16_index,
                    |audio| Nhk16 { audio },
                )
                .await?;
            } else if let Some(path) = path.strip_prefix(SHINMEIKAI8_MEDIA) {
                import_by_index(
                    &mut tx,
                    dictionary_id,
                    &mut scratch,
                    path,
                    &mut entry,
                    &shinmeikai8_index,
                    |audio| Shinmeikai8 { audio },
                )
                .await?;
            }
            anyhow::Ok(())
        })
        .await
        .with_context(|| format!("failed to process `{path}`"))?;

        entries_done += 1;
        if entries_done % 1000 == 0 {
            _ = send_progress.try_send((entries_done as f64) / (num_entries as f64));
        }
    }

    tx.commit().await.context("failed to commit transaction")?;
    Ok(dictionary_id)
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
        &Term::from_headword(headword),
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
struct GenericIndex {
    headwords: HashMap<String, Vec<String>>,
    files: HashMap<String, FileInfo>,
}

#[derive(Debug, Deserialize)]
struct FileInfo {
    // some entries don't have this field
    kana_reading: Option<String>,
}

impl From<GenericIndex> for Index {
    fn from(value: GenericIndex) -> Self {
        let mut path_to_term = HashMap::<String, Term>::new();
        for (headword, paths) in value.headwords {
            for path in paths {
                let term = path_to_term.entry(path).or_default();
                term.headword = Some(headword.clone());
            }
        }
        for (path, info) in value.files {
            let term = path_to_term.entry(path).or_default();
            if let Some(reading) = info.kana_reading {
                term.reading = Some(reading);
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
    index: &Index,
    to_record: impl FnOnce(Bytes) -> T,
) -> Result<()> {
    let Some(term) = index.path_to_term.get(path) else {
        // some files literally just don't have an index entry
        // like Nhk16 `20170616125948.opus`
        trace!("{path} does not have an index entry, skipping");
        return Ok(());
    };

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
struct Nhk16Index(Vec<Nhk16Entry>);

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Nhk16Entry {
    kana: String,
    kanji: Vec<String>,
    accents: Vec<Nhk16Accent>,
    subentries: Vec<Nhk16Subentry>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Nhk16Accent {
    // some entries are `null`
    sound_file: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Nhk16Subentry {
    head: Option<String>,
    accents: Vec<Nhk16Accent>,
}

impl From<Nhk16Index> for Index {
    fn from(value: Nhk16Index) -> Self {
        let mut path_to_term = HashMap::<String, Term>::new();
        for entry in value.0 {
            let kanji = entry
                .kanji
                .into_iter()
                .next()
                .filter(|k| !k.trim().is_empty());
            let term = if let Some(kanji) = kanji {
                Term::new(kanji, &entry.kana)
            } else {
                Term::from_reading(&entry.kana)
            };

            for accent in entry.accents {
                if let Some(path) = accent.sound_file {
                    path_to_term.insert(path, term.clone());
                }
            }

            // TODO: NHK16 also stores `accent[i].pronunciation` which might be useful
            // this format has a ton of useful stuff in general... we should expose it
            for subentry in entry.subentries {
                let headword = subentry.head.or_else(|| term.headword.clone());
                let term = Term {
                    headword,
                    reading: Some(entry.kana.clone()),
                };

                for accent in subentry.accents {
                    if let Some(path) = accent.sound_file {
                        path_to_term.insert(path, term.clone());
                    }
                }
            }
        }
        Self { path_to_term }
    }
}
