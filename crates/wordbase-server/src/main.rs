#![doc = include_str!("../README.md")]

mod dictionary;
mod import;
mod mecab;
mod popup;
mod server;
mod term;
mod texthooker;

use {
    anyhow::{Context, Result},
    futures::TryFutureExt,
    mecab::MecabRequest,
    sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    std::{
        net::{Ipv4Addr, SocketAddr},
        str::FromStr,
        sync::Arc,
        thread,
        time::Duration,
    },
    tokio::{
        sync::{broadcast, mpsc, oneshot},
        task::JoinSet,
    },
    tracing::{Instrument, info, info_span, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{
        DictionaryState,
        hook::HookSentence,
        protocol::{DEFAULT_PORT, LookupConfig, NoRecords, ShowPopupRequest, ShowPopupResponse},
    },
};

const CHANNEL_BUF_CAP: usize = 4;

#[derive(Debug)]
struct Config {
    listen_addr: SocketAddr,
    lookup: LookupConfig,
    texthooker_sources: Vec<Arc<TexthookerSource>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), DEFAULT_PORT),
            lookup: LookupConfig::default(),
            texthooker_sources: vec![Arc::new(TexthookerSource {
                url: "ws://host.docker.internal:9001".into(),
                // url: "ws://127.0.0.1:9001".into(),
                connect_interval: Duration::from_secs(1),
            })],
        }
    }
}

#[derive(Debug)]
struct TexthookerSource {
    url: String,
    connect_interval: Duration,
}

#[derive(Debug, Clone)]
enum ServerEvent {
    HookSentence(HookSentence),
    SyncDictionaries(Vec<DictionaryState>),
}

#[derive(Debug, Clone)]
struct BackendPopupRequest {
    request: ShowPopupRequest,
    send_response: mpsc::Sender<Result<ShowPopupResponse, NoRecords>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    let config = Arc::new(Config::default());

    let db = SqlitePoolOptions::new()
        .max_connections(8)
        .connect_with(
            SqliteConnectOptions::from_str("sqlite://wordbase.db")?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await
        .context("failed to connect to database")?;
    sqlx::query(include_str!("setup_db.sql"))
        .execute(&db)
        .await
        .context("failed to set up database")?;
    info!("Connected to database");

    let (send_mecab_request, recv_mecab_request) = mpsc::channel::<MecabRequest>(CHANNEL_BUF_CAP);
    let (send_server_event, recv_server_event) = broadcast::channel::<ServerEvent>(CHANNEL_BUF_CAP);
    let (send_popup_request, recv_popup_request) =
        broadcast::channel::<BackendPopupRequest>(CHANNEL_BUF_CAP);

    let rt = tokio::runtime::Handle::current();
    let mut tasks = JoinSet::new();

    for source_config in &config.texthooker_sources {
        tasks.spawn(
            texthooker::run(source_config.clone(), send_server_event.clone())
                .instrument(info_span!("texthooker", url = source_config.url))
                .map_err(|err| err.context("texthooker error")),
        );
    }
    tasks.spawn(mecab::run(recv_mecab_request).map_err(|err| err.context("MeCab error")));
    tasks.spawn(
        server::run(
            db.clone(),
            config,
            send_mecab_request,
            send_server_event,
            send_popup_request,
        )
        .map_err(|err| err.context("server error")),
    );
    thread::spawn(move || popup::default::run(db, rt, recv_popup_request, recv_server_event));

    while let Some(result) = tasks.join_next().await {
        result??;
    }
    Ok(())
}
