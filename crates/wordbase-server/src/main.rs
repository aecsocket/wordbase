#![doc = include_str!("../README.md")]

mod dictionary;
mod hook_pull;
mod import;
mod mecab;
mod server;
mod term;

use {
    anyhow::{Context, Result},
    mecab::MecabRequest,
    std::{
        net::{Ipv4Addr, SocketAddr},
        sync::Arc,
        time::Duration,
    },
    tokio::{
        sync::{broadcast, mpsc},
        task::JoinSet,
    },
    tracing::{Instrument, info_span},
    wordbase::{
        Dictionary,
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
                url: "ws://127.0.0.1:9001".into(),
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
    SyncDictionaries(Vec<Dictionary>),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let config = Arc::new(Config::default());

    let (send_mecab_request, recv_mecab_request) = mpsc::channel::<MecabRequest>(CHANNEL_BUF_CAP);
    let (send_event, _) = broadcast::channel::<Event>(CHANNEL_BUF_CAP);

    let mut tasks = JoinSet::new();

    for source_config in &config.texthooker_sources {
        tasks.spawn(
            hook_pull::run(source_config.clone(), send_event.clone())
                .instrument(info_span!("texthooker", url = source_config.url)),
        );
    }
    tasks.spawn(mecab::run(recv_mecab_request));
    tasks.spawn(server::run(config, send_mecab_request, send_event));

    while let Some(result) = tasks.join_next().await {
        result.context("task dropped")??;
    }
    Ok(())
}
