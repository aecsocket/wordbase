mod insert;
mod yomichan_audio;
mod yomitan;

use {
    crate::{CHANNEL_BUF_CAP, DictionaryEvent, Engine, EngineEvent},
    anyhow::{Context, Result},
    derive_more::{Display, Error, From},
    futures::{Stream, TryStreamExt, future::BoxFuture, stream::FuturesUnordered},
    sqlx::{Pool, Sqlite, Transaction},
    std::{
        collections::HashMap,
        path::Path,
        sync::{Arc, LazyLock},
    },
    tokio::{
        fs::File,
        io::{AsyncBufRead, AsyncRead, AsyncSeek, BufReader},
        sync::mpsc,
    },
    tokio_util::task::AbortOnDropHandle,
    tracing::{debug, trace},
    wordbase_api::{DictionaryId, DictionaryKind, DictionaryMeta},
};

static FORMATS: LazyLock<HashMap<DictionaryKind, Arc<dyn ImportKind>>> = LazyLock::new(|| {
    [
        (
            DictionaryKind::Yomitan,
            Arc::new(yomitan::Yomitan) as Arc<dyn ImportKind>,
        ),
        (
            DictionaryKind::YomichanAudio,
            Arc::new(yomichan_audio::YomichanAudio),
        ),
    ]
    .into()
});

pub trait ImportKind: Send + Sync {
    fn is_of_kind(&self, open_archive: Arc<dyn OpenArchive>) -> BoxFuture<'_, Result<()>>;

    fn start_import(
        &self,
        db: Pool<Sqlite>,
        open_archive: Arc<dyn OpenArchive>,
        progress_tx: mpsc::Sender<ImportProgress>,
    ) -> BoxFuture<Result<(DictionaryMeta, ImportContinue)>>;
}

pub trait OpenArchive: Send + Sync {
    fn open_archive(&self) -> BoxFuture<'_, Result<Box<dyn Archive>>>;
}

impl<T, Fut> OpenArchive for T
where
    T: Send + Sync + Fn() -> Fut,
    Fut: Send + Future<Output = Result<Box<dyn Archive>>>,
{
    fn open_archive(&self) -> BoxFuture<'_, Result<Box<dyn Archive>>> {
        Box::pin(async { self().await })
    }
}

impl<P: Send + Sync + AsRef<Path>> OpenArchive for Arc<P> {
    fn open_archive(&self) -> BoxFuture<'_, Result<Box<dyn Archive>>> {
        let path = self.clone();
        Box::pin(async move {
            let file = File::open(&*path).await?;
            Ok(Box::new(BufReader::new(file)) as Box<dyn Archive>)
        })
    }
}

pub trait Archive: Send + Sync + AsyncRead + AsyncSeek + AsyncBufRead + Unpin {}

impl<T: Send + Sync + AsyncRead + AsyncSeek + AsyncBufRead + Unpin> Archive for T {}

pub type ImportContinue = BoxFuture<'static, Result<DictionaryId>>;

#[derive(Debug)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Enum))]
pub enum ImportEvent {
    DeterminedKind(DictionaryKind),
    ParsedMeta(DictionaryMeta),
    Progress(ImportProgress),
    Done(DictionaryId),
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Record))]
pub struct ImportProgress {
    pub frac: f64,
}

#[derive(Debug, Display, Error)]
pub enum GetKindError {
    #[display(
        "archive does not represent a valid dictionary kind\n{}",
        format_errors(_0)
    )]
    NoFormat(#[error(ignore)] HashMap<DictionaryKind, anyhow::Error>),
    #[display("archive represents multiple dictionary kinds: {_0:?}")]
    MultipleFormats(#[error(ignore)] Vec<DictionaryKind>),
}

fn format_errors(errors: &HashMap<DictionaryKind, anyhow::Error>) -> String {
    errors
        .iter()
        .map(|(format, err)| format!("does not represent a `{format:?}` dictionary: {err:?}\n"))
        .collect::<Vec<_>>()
        .join("\n")
}

pub async fn kind_of(
    open_archive: impl Into<Arc<dyn OpenArchive>>,
) -> Result<DictionaryKind, GetKindError> {
    let open_archive = open_archive.into();
    let mut valid_formats = Vec::<DictionaryKind>::new();
    let mut format_errors = HashMap::<DictionaryKind, anyhow::Error>::new();

    let format_results = FORMATS
        .iter()
        .map(|(format, importer)| {
            let open_archive = open_archive.clone();
            async move {
                let result = importer.is_of_kind(open_archive).await;
                Ok((*format, result))
            }
        })
        .collect::<FuturesUnordered<_>>()
        .try_collect::<HashMap<_, _>>()
        .await?;
    for (format, result) in format_results {
        match result {
            Ok(()) => valid_formats.push(format),
            Err(err) => {
                format_errors.insert(format, err);
            }
        }
    }

    match (valid_formats.first(), valid_formats.len()) {
        (None, _) => Err(GetKindError::NoFormat(format_errors)),
        (Some(format), 1) => Ok(*format),
        (_, _) => Err(GetKindError::MultipleFormats(valid_formats)),
    }
}

/// Tracks the state of a dictionary import operation.
#[derive(Debug)]
pub struct ImportStarted {
    pub meta: DictionaryMeta,
    pub progress_rx: mpsc::Receiver<f64>,
}

#[derive(Debug, Display, Error, From)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Error), uniffi(flat_error))]
pub enum ImportError {
    #[display("failed to determine dictionary kind")]
    GetKind(GetKindError),
    #[display("no importer for kind `{kind:?}`")]
    NoImporter { kind: DictionaryKind },
    #[display("failed to parse meta as `{kind:?}`")]
    ParseMeta {
        kind: DictionaryKind,
        source: anyhow::Error,
    },
    #[display("dictionary with this name already exists")]
    AlreadyExists,
    #[display("failed to import as `{kind:?}`")]
    Import {
        kind: DictionaryKind,
        source: anyhow::Error,
    },
    #[from]
    Other(anyhow::Error),
}

impl Engine {
    pub fn import_dictionary(
        &self,
        open_archive: impl OpenArchive + 'static,
    ) -> impl Stream<Item = Result<ImportEvent, ImportError>> {
        self.import_dictionary_arc(Arc::new(open_archive))
    }

    pub fn import_dictionary_arc(
        &self,
        open_archive: Arc<dyn OpenArchive>,
    ) -> impl Stream<Item = Result<ImportEvent, ImportError>> {
        async_stream::try_stream! {
            debug!("Attempting to determine dictionary kind");
            let kind = kind_of(open_archive.clone())
                .await
                .map_err(ImportError::GetKind)?;
            debug!("Importing as {kind:?} dictionary");
            yield ImportEvent::DeterminedKind(kind);

            let importer = FORMATS.get(&kind).ok_or(ImportError::NoImporter { kind })?;
            let (progress_tx, mut progress_rx) = mpsc::channel(CHANNEL_BUF_CAP);

            let (meta, continue_task) = importer
                .start_import(self.db.clone(), open_archive, progress_tx)
                .await
                .map_err(|source| ImportError::ParseMeta { kind, source })?;
            debug!(
                "Importing {:?} dictionary {:?} version {:?}",
                meta.kind, meta.name, meta.version
            );
            let name = meta.name.clone();
            yield ImportEvent::ParsedMeta(meta);

            let already_exists = dictionary_exists_by_name(&self.db, &name)
                .await
                .context("failed to fetch if this dictionary already exists")?;
            if already_exists {
                Err(ImportError::AlreadyExists)?;
                return;
            }
            trace!("Dictionary does not exist yet, spawning import continuation");

            let continue_task = AbortOnDropHandle::new(tokio::spawn(continue_task));
            while let Some(progress) = progress_rx.recv().await {
                yield ImportEvent::Progress(progress);
            }

            let id = continue_task
                .await
                .map_err(|source| ImportError::Import {
                    kind,
                    source: source.into(),
                })?
                .map_err(|source| ImportError::Import { kind, source })?;

            self.sync_dictionaries().await?;
            _ = self
                .event_tx
                .send(EngineEvent::Dictionary(DictionaryEvent::Added { id }));
            yield ImportEvent::Done(id);
        }
    }
}

async fn dictionary_exists_by_name(db: &Pool<Sqlite>, name: &str) -> Result<bool> {
    let result = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM dictionary WHERE json_extract(meta, '$.name') = $1)",
        name
    )
    .fetch_one(db)
    .await?;
    Ok(result > 0)
}

async fn insert_dictionary(
    tx: &mut Transaction<'_, Sqlite>,
    meta: &DictionaryMeta,
) -> Result<DictionaryId> {
    let meta = serde_json::to_string(meta).context("failed to serialize dictionary meta")?;
    let id = sqlx::query!(
        "INSERT INTO dictionary (meta, position)
        VALUES ($1, (SELECT COALESCE(MAX(position), 0) + 1 FROM dictionary))",
        meta
    )
    .execute(&mut **tx)
    .await?
    .last_insert_rowid();
    Ok(DictionaryId(id))
}

#[cfg(feature = "uniffi")]
const _: () = {
    use {
        crate::{FfiResult, Wordbase},
        std::os::fd::{FromRawFd, RawFd},
        tokio::{fs::File, io::BufReader},
    };

    #[uniffi::export(with_foreign)]
    pub trait ImportDictionaryCallback: Send + Sync {
        fn open_archive_file(&self) -> FfiResult<RawFd>;

        fn on_event(&self, event: ImportEvent) -> FfiResult<()>;
    }

    #[uniffi::export(async_runtime = "tokio")]
    impl Wordbase {
        pub async fn import_dictionary(
            &self,
            callback: Arc<dyn ImportDictionaryCallback>,
        ) -> FfiResult<DictionaryId> {
            let events = self.0.import_dictionary({
                let callback = callback.clone();
                move || {
                    let callback = callback.clone();
                    async move {
                        let fd = callback.open_archive_file()?;
                        // SAFETY: it is the FFI caller's responsibility
                        // to ensure that this fd is valid and open
                        let file = unsafe { File::from_raw_fd(fd) };
                        Ok(Box::new(BufReader::new(file)) as Box<dyn Archive>)
                    }
                }
            });
            tokio::pin!(events);
            while let Some(event) = events.try_next().await.map_err(anyhow::Error::new)? {
                match event {
                    ImportEvent::Done(id) => return Ok(id),
                    event => {
                        _ = callback.on_event(event);
                    }
                }
            }
            unreachable!();
        }
    }
};
