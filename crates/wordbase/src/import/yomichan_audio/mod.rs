mod schema;

use {
    super::{ImportContinue, ImportKind, insert::Insert},
    crate::import::insert_dictionary,
    anyhow::{Context, Result, bail},
    async_compression::futures::bufread::XzDecoder,
    async_tar::EntryType,
    bytes::Bytes,
    derive_more::Deref,
    foldhash::{HashMap, HashMapExt},
    futures::{
        AsyncBufRead, AsyncRead, AsyncReadExt as _, AsyncSeek, AsyncSeekExt as _, StreamExt,
        future::BoxFuture,
    },
    pin_project::pin_project,
    schema::{
        FORVO_PATH, JPOD_INDEX, JPOD_MEDIA, MARKER_PATHS, NHK16_AUDIO, NHK16_INDEX,
        SHINMEIKAI8_INDEX, SHINMEIKAI8_MEDIA,
    },
    serde::de::DeserializeOwned,
    sqlx::{Pool, Sqlite, Transaction},
    std::{
        any::type_name,
        io::SeekFrom,
        path::Path,
        pin::Pin,
        sync::{
            Arc,
            atomic::{self, AtomicU64},
        },
        task::Poll,
    },
    tokio::{fs::File, io::BufReader, sync::mpsc},
    tokio_util::compat::{Compat, TokioAsyncReadCompatExt},
    tracing::{debug, trace},
    wordbase_api::{
        DictionaryId, DictionaryKind, DictionaryMeta, NormString, Record, RecordType, Term,
        dict::{
            jpn::PitchPosition,
            yomichan_audio::{Audio, AudioFormat, Forvo, Jpod, Nhk16, Shinmeikai8},
        },
    },
};

pub struct YomichanAudio;

impl ImportKind for YomichanAudio {
    fn is_of_kind(&self, archive_path: Arc<Path>) -> BoxFuture<'_, Result<()>> {
        Box::pin(validate(archive_path))
    }

    fn start_import(
        &self,
        db: Pool<Sqlite>,
        archive_path: Arc<Path>,
        progress_tx: mpsc::Sender<f64>,
    ) -> BoxFuture<Result<(DictionaryMeta, ImportContinue)>> {
        Box::pin(async move {
            let mut meta = DictionaryMeta::new(
                DictionaryKind::YomichanAudio,
                "Yomichan Japanese Local Audio",
            );
            meta.url = Some("https://github.com/yomidevs/local-audio-yomichan".into());
            Ok((
                meta.clone(),
                Box::pin(import(db, archive_path, meta, progress_tx)) as ImportContinue,
            ))
        })
    }
}

async fn open_archive(
    archive_path: &Path,
) -> Result<(
    async_tar::Archive<XzDecoder<Count<Compat<BufReader<File>>>>>,
    Arc<AtomicU64>,
    u64,
)> {
    let mut reader = BufReader::new(
        File::open(archive_path)
            .await
            .context("failed to open file")?,
    )
    .compat();
    let buf_len = reader
        .seek(SeekFrom::End(0))
        .await
        .context("failed to seek to end of file")?;
    reader
        .seek(SeekFrom::Start(0))
        .await
        .context("failed to seek to start of file")?;

    let count = Count::new(reader);
    let cursor_pos = count.pos();
    let archive = async_tar::Archive::new(XzDecoder::new(count));
    Ok((archive, cursor_pos, buf_len))
}

async fn validate(archive_path: Arc<Path>) -> Result<()> {
    let (archive, _, _) = open_archive(&archive_path).await?;
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

#[pin_project]
struct Count<T> {
    #[pin]
    inner: T,
    pos: Arc<AtomicU64>,
}

impl<T> Count<T> {
    pub fn new(inner: T) -> Self {
        Self {
            inner,
            pos: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn pos(&self) -> Arc<AtomicU64> {
        self.pos.clone()
    }
}

impl<T: AsyncSeek + Unpin> AsyncSeek for Count<T> {
    fn poll_seek(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        pos: std::io::SeekFrom,
    ) -> Poll<std::io::Result<u64>> {
        self.project().inner.poll_seek(cx, pos)
    }
}

impl<T: AsyncRead + Unpin> AsyncRead for Count<T> {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> Poll<std::io::Result<usize>> {
        self.project().inner.poll_read(cx, buf)
    }

    fn poll_read_vectored(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        bufs: &mut [std::io::IoSliceMut<'_>],
    ) -> Poll<std::io::Result<usize>> {
        self.project().inner.poll_read_vectored(cx, bufs)
    }
}

impl<T: AsyncBufRead + Unpin> AsyncBufRead for Count<T> {
    fn poll_fill_buf<'a>(
        self: Pin<&'a mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<std::io::Result<&'a [u8]>> {
        self.project().inner.poll_fill_buf(cx)
    }

    fn consume(self: Pin<&mut Self>, amt: usize) {
        let this = self.project();
        this.pos.fetch_add(amt as u64, atomic::Ordering::SeqCst);
        this.inner.consume(amt);
    }
}

async fn import(
    db: Pool<Sqlite>,
    archive_path: Arc<Path>,
    meta: DictionaryMeta,
    progress_tx: mpsc::Sender<f64>,
) -> Result<DictionaryId> {
    let mut tx = db.begin().await.context("failed to begin transaction")?;
    let mut records = Insert::<Record>::new();
    let dictionary_id = insert_dictionary(&mut tx, &meta)
        .await
        .context("failed to insert dictionary")?;

    debug!("Counting entries and parsing indexes");
    let mut jpod_rev_index = None::<RevIndex<GenericInfo>>;
    let mut nhk16_rev_index = None::<RevIndex<Nhk16Info>>;
    let mut shinmeikai8_rev_index = None::<RevIndex<GenericInfo>>;

    let (archive, cursor_pos, buf_len) = open_archive(&archive_path).await?;
    let mut entries = archive
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
        if num_entries % 2000 == 0 {
            let cursor_pos = cursor_pos.load(atomic::Ordering::SeqCst);
            let progress = (cursor_pos as f64) / (buf_len as f64);
            _ = progress_tx.try_send(progress * 0.5);
        }
    }
    debug!("{num_entries} total entries");

    let jpod_rev_index =
        jpod_rev_index.with_context(|| format!("no JPod index at `{JPOD_INDEX}`"))?;
    let nhk16_rev_index =
        nhk16_rev_index.with_context(|| format!("no NHK index at `{NHK16_INDEX}`"))?;
    let shinmeikai8_rev_index = shinmeikai8_rev_index
        .with_context(|| format!("no Shinmeikai index at `{SHINMEIKAI8_INDEX}`"))?;

    let (archive, _, _) = open_archive(&archive_path).await?;
    let mut entries = archive
        .entries()
        .context("failed to read archive entries")?;
    let mut entries_done = 0usize;
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
                import_forvo(&mut tx, &mut records, dictionary_id, path, &mut entry)
                    .await
                    .context("failed to import Forvo file")?;
            } else if let Some(path) = path.strip_prefix(JPOD_MEDIA) {
                import_by_rev_index(
                    &mut tx,
                    &mut records,
                    dictionary_id,
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
                    &mut records,
                    dictionary_id,
                    path,
                    &mut entry,
                    &nhk16_rev_index,
                    |info| info.terms.iter(),
                    |audio, info| Nhk16 {
                        audio,
                        pitch_positions: info
                            .pitch_positions
                            .iter()
                            .copied()
                            .map(PitchPosition)
                            .collect(),
                    },
                )
                .await?;
            } else if let Some(path) = path.strip_prefix(SHINMEIKAI8_MEDIA) {
                import_by_rev_index(
                    &mut tx,
                    &mut records,
                    dictionary_id,
                    path,
                    &mut entry,
                    &shinmeikai8_rev_index,
                    |info| info.term.as_ref().into_iter(),
                    |audio, info| Shinmeikai8 {
                        audio,
                        pitch_number: info.pitch_number.map(PitchPosition),
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
        if entries_done % 2000 == 0 {
            let progress = (entries_done as f64) / (num_entries as f64);
            _ = progress_tx.try_send(progress.mul_add(0.5, 0.5));
        }
    }

    records
        .flush(&mut tx)
        .await
        .context("failed to flush records")?;
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
    pitch_pattern: Option<NormString>,
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
            let entry = for_path.entry(path).or_default();

            if let Some(reading) = info.kana_reading.and_then(NormString::new) {
                match &mut entry.term {
                    Some(term) => {
                        term.set_reading(reading);
                    }
                    None => {
                        entry.term = Term::from_reading(reading);
                    }
                }
            }

            if let Some(pitch_pattern) = info.pitch_pattern.and_then(NormString::new) {
                entry.pitch_pattern = Some(pitch_pattern);
            }

            if let Some(pitch_number) = info.pitch_number.and_then(|s| s.parse::<u64>().ok()) {
                entry.pitch_number = Some(pitch_number);
            }
        }
        Ok(Self { for_path })
    }
}

#[derive(Debug, Default)]
struct Nhk16Info {
    terms: Vec<Term>,
    pitch_positions: Vec<u64>,
}

impl TryFrom<schema::nhk16::Index> for RevIndex<Nhk16Info> {
    type Error = anyhow::Error;

    fn try_from(value: schema::nhk16::Index) -> Result<Self, Self::Error> {
        let mut for_path = HashMap::<String, Nhk16Info>::new();
        for entry in value.0 {
            let reading = NormString::new(entry.kana);
            let terms = entry
                .kanji
                .into_iter()
                .filter_map(NormString::new)
                .filter_map(|headword| Term::from_parts(Some(headword), reading.clone()))
                .collect::<Vec<_>>();

            for accent in entry.accents {
                let Some(sound_file) = accent.sound_file else {
                    continue;
                };
                let entry = for_path.entry(sound_file).or_default();
                entry.terms.extend_from_slice(&terms);

                entry.pitch_positions.extend(
                    accent
                        .accent
                        .iter()
                        .filter_map(|accent| u64::try_from(accent.pitch_accent).ok()),
                );
            }

            // subentries are usually just conjugations of top-level entries,
            // so we ignore them
        }
        Ok(Self { for_path })
    }
}

pub async fn import_forvo<R: AsyncRead + Unpin>(
    tx: &mut Transaction<'_, Sqlite>,
    records: &mut Insert<Record>,
    source: DictionaryId,
    path: &str,
    entry: &mut async_tar::Entry<R>,
) -> Result<()> {
    let mut parts = path.split('/');
    let username = parts
        .next()
        .map(ToOwned::to_owned)
        .context("no Forvo username in path")?;
    let term = parts
        .next()
        .and_then(|part| part.rsplit_once('.'))
        .and_then(|(name, _)| Term::from_headword(name))
        .context("no headword in path")?;

    records
        .insert(
            tx,
            source,
            term,
            &Forvo {
                username,
                audio: Audio {
                    format: format_of(path)?,
                    data: encode(entry).await?,
                },
            },
        )
        .await
        .context("failed to insert record")?;
    Ok(())
}

fn format_of(path: &str) -> Result<AudioFormat> {
    Ok(match Path::new(path).extension().map(|s| s.to_str()) {
        Some(Some("opus")) => AudioFormat::Opus,
        Some(Some("mp3")) => AudioFormat::Mp3,
        Some(Some(ext)) => bail!("unknown audio format `{ext}`"),
        _ => bail!("invalid file extension"),
    })
}

#[expect(clippy::future_not_send, reason = "we don't care about non-send here")]
async fn import_by_rev_index<'a, R, Rev, T, Terms>(
    tx: &mut Transaction<'_, Sqlite>,
    records: &mut Insert<Record>,
    source: DictionaryId,
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
        trace!(
            "{path} of type `{}` does not have an index entry, skipping",
            type_name::<T>()
        );
        return Ok(());
    };

    let audio = Audio {
        format: format_of(path)?,
        data: encode(entry).await?,
    };
    let record = into_record(audio, info);
    for term in terms_of(info) {
        records
            .insert(tx, source, term.clone(), &record)
            .await
            .context("failed to insert record")?;
    }
    Ok(())
}

async fn encode<R>(entry: &mut async_tar::Entry<R>) -> Result<Bytes>
where
    R: AsyncRead + Unpin,
{
    let mut scratch = Vec::new();
    entry
        .read_to_end(&mut scratch)
        .await
        .context("failed to read audio data into memory")?;
    Ok(Bytes::from(scratch))
}
