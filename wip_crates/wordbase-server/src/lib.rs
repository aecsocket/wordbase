#![doc = include_str!("../README.md")]

use {
    anyhow::{Context, Result},
    futures::never::Never,
    tokio::net::{TcpListener, TcpStream, ToSocketAddrs},
    tracing::{Instrument, info, info_span},
    wordbase_engine::Engine,
};

pub async fn start(engine: Engine, addr: impl ToSocketAddrs) -> Result<Never> {
    let listener = TcpListener::bind(addr)
        .await
        .context("failed to bind socket")?;

    loop {
        let (stream, peer_addr) = listener.accept().await.context("failed to accept stream")?;

        let engine = engine.clone();
        tokio::spawn(
            async move {
                info!("Connecting");
                let Err(err) = accept_stream(engine, stream).await;
                info!("Disconnected: {err:?}");
            }
            .instrument(info_span!("connection", peer = ?peer_addr)),
        );
    }
}

async fn accept_stream(engine: Engine, stream: TcpStream) -> Result<Never> {
    let stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to accept WebSocket connection")?;

    loop {}
}
