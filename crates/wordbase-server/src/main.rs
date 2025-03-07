#![doc = include_str!("../README.md")]

pub(crate) mod db;
pub(crate) mod import;
mod mecab;
mod server;
mod textractor;

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time::Duration,
};

use anyhow::Result;
use mecab::MecabRequest;
use tokio::sync::{broadcast, mpsc};
use wordbase::{DEFAULT_PORT, Dictionary, lookup::LookupConfig, protocol::NewSentence};

const CHANNEL_BUF_CAP: usize = 4;

#[derive(Debug)]
struct Config {
    listen_addr: SocketAddr,
    textractor_url: String,
    textractor_connect_interval: Duration,
    lookup: LookupConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_PORT),
            textractor_url: "ws://127.0.0.1:9001".into(),
            textractor_connect_interval: Duration::from_secs(1),
            lookup: LookupConfig::default(),
        }
    }
}

#[derive(Debug, Clone)]
enum Event {
    NewSentence(NewSentence),
    SyncDictionaries(Vec<Dictionary>),
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let config = Arc::new(Config::default());

    let (send_mecab_request, recv_mecab_request) = mpsc::channel::<MecabRequest>(CHANNEL_BUF_CAP);
    let (send_event, _) = broadcast::channel::<Event>(CHANNEL_BUF_CAP);

    #[expect(
        unreachable_code,
        reason = "macro generates code which reads values in uninhabited types"
    )]
    tokio::try_join!(
        mecab::run(recv_mecab_request),
        textractor::run(config.clone(), send_event.clone()),
        server::run(config.clone(), send_mecab_request, send_event),
    )?;
    Ok(())
}
