#![doc = include_str!("../README.md")]

mod mecab;
mod websocket;

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use anyhow::Result;
use mecab::MecabRequest;
use tokio::sync::mpsc;
use wordbase::{DEFAULT_PORT, lookup::LookupConfig};

/// Wordbase server.
#[derive(Debug, clap::Parser)]
struct Args {
    /// Socket address to listen for connections on.
    #[arg(long, default_value_t = DEFAULT_LISTEN_ADDR)]
    listen_addr: SocketAddr,
}

#[derive(Debug)]
struct Config {
    listen_addr: SocketAddr,
    lookup: LookupConfig,
}

const DEFAULT_LISTEN_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_PORT);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();
    let args = <Args as clap::Parser>::parse();

    let config = Arc::new(Config {
        listen_addr: DEFAULT_LISTEN_ADDR,
        lookup: LookupConfig::default(),
    });
    let (send_mecab_request, recv_mecab_request) = mpsc::channel::<MecabRequest>(4);

    tokio::try_join!(
        mecab::run(recv_mecab_request),
        websocket::run(config, send_mecab_request),
    )?;
    Ok(())
}
