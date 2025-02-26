#![doc = include_str!("../README.md")]

use core::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    num::Wrapping,
};

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt, never::Never};
use tokio::net::{TcpListener, TcpStream};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Message, Utf8Bytes,
        protocol::{CloseFrame, frame::coding::CloseCode},
    },
};
use tracing::{Instrument, info, info_span, trace};
use wordbase::{DEFAULT_PORT, response};

/// Wordbase server.
#[derive(Debug, clap::Parser)]
struct Args {
    /// Socket address to listen for connections on.
    #[arg(long, default_value_t = DEFAULT_LISTEN_ADDR)]
    listen_addr: SocketAddr,
}

const DEFAULT_LISTEN_ADDR: SocketAddr =
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_PORT);

#[tokio::main]
async fn main() -> Result<Never> {
    tracing_subscriber::fmt().init();
    let args = <Args as clap::Parser>::parse();

    let listener = TcpListener::bind(&args.listen_addr)
        .await
        .context("failed to bind TCP listener")?;
    info!("Listening on {:?}", args.listen_addr);

    let mut connection_id = Wrapping(0usize);
    loop {
        let (stream, peer_addr) = listener
            .accept()
            .await
            .context("failed to accept TCP stream")?;

        tokio::spawn(
            async move {
                info!("Incoming connection from {peer_addr:?}");
                let Err(err) = handle_stream(stream).await;
                info!("Connection lost: {err:?}");
            }
            .instrument(info_span!("connection", id = %connection_id)),
        );

        connection_id += 1;
    }
}

async fn handle_stream(stream: TcpStream) -> Result<Never> {
    let mut stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to accept WebSocket stream")?;

    loop {
        let message = stream
            .next()
            .await
            .context("stream closed")?
            .context("stream error")?;

        if let Err(err) = handle_message(&mut stream, message).await {
            let close_frame = CloseFrame {
                code: CloseCode::Abnormal,
                reason: Utf8Bytes::from(err.to_string()),
            };
            tokio::spawn(async move {
                _ = stream.close(Some(close_frame)).await;
            });
            return Err(err);
        }
    }
}

async fn handle_message(stream: &mut WebSocketStream<TcpStream>, message: Message) -> Result<()> {
    let message = message
        .into_text()
        .context("received message which is not UTF-8 text")?;
    if message.is_empty() {
        return Ok(());
    }

    let request =
        serde_json::from_str::<wordbase::Request>(&message).context("received invalid request")?;

    let response: wordbase::Response = match request {
        wordbase::Request::Ping => {
            trace!("Requested ping");
            response::Pong {
                version: option_env!("CARGO_PKG_VERSION").unwrap_or("?").into(),
            }
            .into()
        }
        wordbase::Request::Lookup(lookup) => {
            info!("Requested lookup for {:?}", lookup.text);
            response::Lookup {
                chars_matched: 3,
                entries: format!("you sent: {}", lookup.text),
            }
            .into()
        }
    };

    let response = serde_json::to_string(&response).context("failed to serialize response")?;
    stream
        .send(Message::from(response))
        .await
        .context("failed to send response")?;

    Ok(())
}
