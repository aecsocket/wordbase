//! Headless server binary used for testing.

use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

use anyhow::{Context as _, Result};
use tracing::info;
use tracing_subscriber::{EnvFilter, filter::LevelFilter};
use wordbase_engine::Engine;

/// `wordbase-server` standalone binary
///
/// Use this for development purposes or testing without a GUI app.
/// This server does not support all features, such as any features which
/// integrate with the window manager, like popup dictionaries.
#[derive(clap::Parser)]
struct Args {
    /// Path to the engine data directory, which must contain `wordbase.db`
    #[arg(short, long)]
    data_dir: Option<PathBuf>,
    /// Socket address to bind to.
    #[arg(short, long, default_value_t = DEFAULT_BIND_ADDR)]
    bind_addr: SocketAddr,
}

const DEFAULT_BIND_ADDR: SocketAddr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 9518);

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    let args = <Args as clap::Parser>::parse();

    let data_dir = if let Some(data_dir) = args.data_dir {
        data_dir
    } else {
        wordbase_engine::data_dir().context("failed to get default data directory")?
    };
    let bind_addr = args.bind_addr;

    let engine = Engine::new(data_dir)
        .await
        .context("failed to create engine")?;
    info!("");
    info!(
        "    {} v{}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );
    info!("    http://{bind_addr}/api/docs");
    info!("");
    wordbase_server::run(engine, bind_addr).await
}
