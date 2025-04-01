mod schema;

use {
    super::{ImportContinue, ImportKind, ImportStarted, insert_record, insert_term_record},
    crate::{CHANNEL_BUF_CAP, Engine, import::insert_dictionary},
    anyhow::{Context, Result, bail},
    async_compression::futures::bufread::XzDecoder,
    async_tar::EntryType,
    bytes::Bytes,
    derive_more::Deref,
    foldhash::{HashMap, HashMapExt},
    futures::{AsyncRead, AsyncReadExt, StreamExt, future::BoxFuture, io::Cursor},
    schema::{
        FORVO_PATH, JPOD_INDEX, JPOD_MEDIA, MARKER_PATHS, NHK16_AUDIO, NHK16_INDEX,
        SHINMEIKAI8_INDEX, SHINMEIKAI8_MEDIA,
    },
    serde::de::DeserializeOwned,
    sqlx::{Sqlite, Transaction},
    std::{
        pin::Pin,
        sync::{Arc, atomic::AtomicU64},
    },
    tokio::sync::mpsc,
    tracing::{debug, trace},
    wordbase::{
        DictionaryId, DictionaryKind, DictionaryMeta, NonEmptyString, RecordType, Term,
        dict::yomichan_audio::{Audio, AudioFormat, Forvo, Jpod, Nhk16, Shinmeikai8},
    },
};

pub struct YomichanAudio;

impl ImportKind for YomichanAudio {
    fn is_of_kind(&self, archive: Bytes) -> BoxFuture<'_, Result<()>> {
        Box::pin(validate(archive))
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

async fn validate(archive: Bytes) -> Result<()> {
    let archive = async_tar::Archive::new(XzDecoder::new(Cursor::new(archive)));
    let mut entries = archive
        .entries()
        .context("failed to read archive entries")?;
    while let Some(entry) = entries.next().await {
        let entry = entry.context("failed to read entry")?;
        let path = entry.path().context("failed to read entry path")?;
        let path = path
            .to_str()
            .with_context(|| format!("path {path:?} is not UTF-8"))?;

        if MARKER_PATHS.contains(&path) {
            return Ok(());
        }
    }
    bail!("missing one of {MARKER_PATHS:?}");
}

struct CountCursor<T> {
    inner: Pin<Box<Cursor<T>>>,
    pos: Arc<AtomicU64>,
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
    let mut jpod_rev_index = None::<RevIndex<GenericInfo>>;
    let mut nhk16_rev_index = None::<RevIndex<Nhk16Info>>;
    let mut shinmeikai8_rev_index = None::<RevIndex<GenericInfo>>;
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
                    jpod_rev_index = Some(
                        parse_rev_index::<_, schema::generic::Index, GenericInfo>(&mut entry)
                            .await?,
                    );
                }
                NHK16_INDEX => {
                    nhk16_rev_index = Some(
                        parse_rev_index::<_, schema::nhk16::Index, Nhk16Info>(&mut entry).await?,
                    );
                }
                SHINMEIKAI8_INDEX => {
                    shinmeikai8_rev_index = Some(
                        parse_rev_index::<_, schema::generic::Index, GenericInfo>(&mut entry)
                            .await?,
                    );
                }
                _ => {}
            }
            anyhow::Ok(())
        })
        .await
        .with_context(|| format!("failed to process `{path}`"))?;

        num_entries += 1;
    }
    debug!("{num_entries} total entries");

    let jpod_rev_index =
        jpod_rev_index.with_context(|| format!("no JPod index at `{JPOD_INDEX}`"))?;
    let nhk16_rev_index =
        nhk16_rev_index.with_context(|| format!("no NHK index at `{NHK16_INDEX}`"))?;
    let shinmeikai8_rev_index = shinmeikai8_rev_index
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
                import_by_rev_index(
                    &mut tx,
                    dictionary_id,
                    &mut scratch,
                    path,
                    &mut entry,
                    &jpod_rev_index,
                    |info| info.term.as_ref().into_iter(),
                    |audio, _info| Jpod { audio },
                )
                .await?;
            } else if let Some(path) = path.strip_prefix(NHK16_AUDIO) {
                import_by_rev_index(
                    &mut tx,
                    dictionary_id,
                    &mut scratch,
                    path,
                    &mut entry,
                    &nhk16_rev_index,
                    |info| info.terms.iter(),
                    |audio, _info| Nhk16 { audio },
                )
                .await?;
            } else if let Some(path) = path.strip_prefix(SHINMEIKAI8_MEDIA) {
                import_by_rev_index(
                    &mut tx,
                    dictionary_id,
                    &mut scratch,
                    path,
                    &mut entry,
                    &shinmeikai8_rev_index,
                    |info| info.term.as_ref().into_iter(),
                    |audio, info| Shinmeikai8 {
                        audio,
                        pitch_number: info.pitch_number,
                        pitch_pattern: info.pitch_pattern.clone(),
                    },
                )
                .await?;
            }
            anyhow::Ok(())
        })
        .await
        .with_context(|| format!("failed to process `{path}`"))?;

        entries_done += 1;
        if entries_done % 1000 == 0 {
            let progress = (entries_done as f64) / (num_entries as f64);
            _ = send_progress.try_send(progress);
        }
    }

    tx.commit().await.context("failed to commit transaction")?;
    Ok(dictionary_id)
}

#[derive(Debug, Deref)]
struct RevIndex<Rev> {
    for_path: HashMap<String, Rev>,
}

async fn parse_rev_index<R, Fwd, Rev>(entry: &mut async_tar::Entry<R>) -> Result<RevIndex<Rev>>
where
    Fwd: DeserializeOwned + TryInto<RevIndex<Rev>, Error = anyhow::Error>,
    R: AsyncRead + Unpin,
{
    let mut index = Vec::new();
    entry
        .read_to_end(&mut index)
        .await
        .context("failed to read file into memory")?;
    let index = serde_json::from_slice::<Fwd>(&index).context("failed to parse forward index")?;
    let rev_index = index.try_into().context("failed to create reverse index")?;
    Ok(rev_index)
}

#[derive(Debug, Default)]
struct GenericInfo {
    term: Option<Term>,
    pitch_pattern: Option<NonEmptyString>,
    pitch_number: Option<u64>,
}

impl TryFrom<schema::generic::Index> for RevIndex<GenericInfo> {
    type Error = anyhow::Error;

    fn try_from(value: schema::generic::Index) -> Result<Self, Self::Error> {
        let mut for_path = HashMap::<String, GenericInfo>::new();
        for (headword, paths) in value.headwords {
            let Some(term) = Term::from_headword(headword) else {
                continue;
            };
            for path in paths {
                let entry = for_path.entry(path).or_default();
                entry.term = Some(term.clone());
            }
        }
        for (path, info) in value.files {
            let reading = info.kana_reading.and_then(NonEmptyString::new);
            let pitch_pattern = info.pitch_pattern.and_then(NonEmptyString::new);
            let pitch_number = info.pitch_number.and_then(|s| s.parse::<u64>().ok());

            let entry = for_path.entry(path).or_default();
            match &mut entry.term {
                Some(term) => {
                    if let Some(reading) = reading {
                        term.set_reading(reading);
                    }
                }
                None => {
                    entry.term = Term::from_reading(reading);
                }
            };
            if let Some(pitch_pattern) = pitch_pattern {
                entry.pitch_pattern = Some(pitch_pattern);
            }
            if let Some(pitch_number) = pitch_number {
                entry.pitch_number = Some(pitch_number);
            }
        }
        Ok(Self { for_path })
    }
}

#[derive(Debug, Default)]
struct Nhk16Info {
    terms: Vec<Term>,
}

impl TryFrom<schema::nhk16::Index> for RevIndex<Nhk16Info> {
    type Error = anyhow::Error;

    fn try_from(value: schema::nhk16::Index) -> Result<Self, Self::Error> {
        let mut for_path = HashMap::<String, Nhk16Info>::new();
        for entry in value.0 {
            let reading = NonEmptyString::new(entry.kana);
            let terms = entry
                .kanji
                .into_iter()
                .filter_map(NonEmptyString::new)
                .filter_map(|headword| Term::new(headword, reading.clone()))
                .collect::<Vec<_>>();

            for accent in entry.accents {
                if let Some(sound_file) = accent.sound_file {
                    for_path
                        .entry(sound_file)
                        .or_default()
                        .terms
                        .extend_from_slice(&terms);
                }
            }

            // subentries are usually just conjugations of top-level entries,
            // so we ignore them
        }
        Ok(Self { for_path })
    }
}

pub async fn import_forvo<R: AsyncRead + Unpin>(
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
    let headword = parts
        .next()
        .and_then(|part| part.rsplit_once('.'))
        .and_then(|(name, _)| Term::from_headword(name))
        .context("no headword in path")?;

    let mut data = Vec::new();
    entry
        .read_to_end(&mut data)
        .await
        .context("failed to read audio data into memory")?;

    let record_id = insert_record(
        tx,
        source,
        &Forvo {
            username,
            audio: Audio {
                format: AudioFormat::Opus,
                data: Bytes::from(data),
            },
        },
        scratch,
    )
    .await
    .context("failed to insert record")?;
    insert_term_record(tx, source, record_id, &headword)
        .await
        .context("failed to insert headword term")?;
    Ok(())
}

#[expect(clippy::future_not_send, reason = "we don't care about non-send here")]
async fn import_by_rev_index<'a, R, Rev, T, Terms>(
    tx: &mut Transaction<'_, Sqlite>,
    source: DictionaryId,
    scratch: &mut Vec<u8>,
    path: &str,
    entry: &mut async_tar::Entry<R>,
    index: &'a RevIndex<Rev>,
    terms_of: impl FnOnce(&'a Rev) -> Terms,
    into_record: impl FnOnce(Audio, &Rev) -> T,
) -> Result<()>
where
    R: AsyncRead + Unpin,
    T: RecordType,
    Terms: Iterator<Item = &'a Term>,
{
    let Some(info) = index.for_path.get(path) else {
        // some files literally just don't have an index entry
        // like NHK `20170616125948.opus`
        trace!("{path} does not have an index entry, skipping");
        return Ok(());
    };

    let mut data = Vec::new();
    entry
        .read_to_end(&mut data)
        .await
        .context("failed to read audio data into memory")?;

    let audio = Audio {
        format: AudioFormat::Opus,
        data: Bytes::from(data),
    };
    let record = into_record(audio, info);
    let record_id = insert_record(tx, source, &record, scratch)
        .await
        .context("failed to insert record")?;

    for term in terms_of(info) {
        insert_term_record(tx, source, record_id, term)
            .await
            .context("failed to insert term record")?;
    }
    Ok(())
}
