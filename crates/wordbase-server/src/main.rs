#![doc = include_str!("../README.md")]

mod dictionary;
mod import;
mod mecab;
mod popup;
mod server;
mod term;
mod texthooker;

use {
    anyhow::Result,
    mecab::MecabRequest,
    popup::DefaultPopups,
    std::{
        net::{Ipv4Addr, SocketAddr},
        sync::Arc,
        time::Duration,
    },
    tokio::{
        sync::{broadcast, mpsc},
        task::JoinSet,
    },
    tracing::{Instrument, info_span, level_filters::LevelFilter},
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
enum Event {
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
    let popups = Arc::new(DefaultPopups::new());

    let (send_mecab_request, recv_mecab_request) = mpsc::channel::<MecabRequest>(CHANNEL_BUF_CAP);
    let (send_event, _) = broadcast::channel::<Event>(CHANNEL_BUF_CAP);

    let mut tasks = JoinSet::new();

    for source_config in &config.texthooker_sources {
        tasks.spawn(
            texthooker::run(source_config.clone(), send_event.clone())
                .instrument(info_span!("texthooker", url = source_config.url)),
        );
    }
    tasks.spawn(mecab::run(recv_mecab_request));
    tasks.spawn(server::run(config, popups, send_mecab_request, send_event));

    while let Some(result) = tasks.join_next().await {
        result??;
    }
    Ok(())
}
