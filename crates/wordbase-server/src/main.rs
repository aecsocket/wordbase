#![doc = include_str!("../README.md")]

mod db;
mod import;
mod lookup;
mod popup;
mod server;
mod texthooker;

use {
    anyhow::{Context, Result},
    futures::TryFutureExt,
    sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    std::{
        net::{Ipv4Addr, SocketAddr},
        str::FromStr,
        sync::Arc,
        time::Duration,
    },
    tokio::{
        sync::{Semaphore, broadcast},
        task::JoinSet,
    },
    tracing::{Instrument, info, info_span, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{
        DictionaryState,
        hook::HookSentence,
        protocol::{DEFAULT_PORT, LookupConfig},
    },
};

const CHANNEL_BUF_CAP: usize = 4;

#[derive(Debug)]
struct Config {
    listen_addr: SocketAddr,
    lookup: LookupConfig,
    max_db_connections: u32,
    max_concurrent_imports: usize,
    texthooker_sources: Vec<Arc<TexthookerSource>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: SocketAddr::new(Ipv4Addr::LOCALHOST.into(), DEFAULT_PORT),
            lookup: LookupConfig::default(),
            max_db_connections: 8,
            max_concurrent_imports: 4,
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

    info!("Setting up database");
    let db = SqlitePoolOptions::new()
        .max_connections(config.max_db_connections)
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

    let (send_server_event, recv_server_event) = broadcast::channel::<ServerEvent>(CHANNEL_BUF_CAP);
    let mut tasks = JoinSet::new();

    let (lookups, lookup_task) = lookup::Client::new(config.clone(), db.clone());
    let popups = popup::Client::new(lookups.clone(), recv_server_event);

    tasks.spawn(lookup_task.map_err(|err| err.context("lookup task error")));
    for source_config in &config.texthooker_sources {
        tasks.spawn(
            texthooker::run(source_config.clone(), send_server_event.clone())
                .instrument(info_span!("texthooker", url = source_config.url))
                .map_err(|err| err.context("texthooker error")),
        );
    }
    tasks.spawn(
        server::run(server::State {
            db,
            lookups,
            popups,
            send_event: send_server_event,
            concurrent_imports: Arc::new(Semaphore::new(config.max_concurrent_imports)),
            config,
        })
        .map_err(|err| err.context("server error")),
    );

    while let Some(result) = tasks.join_next().await {
        result??;
    }
    Ok(())
}
