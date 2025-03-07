use std::sync::Arc;

use anyhow::{Context, Result};
use futures::{StreamExt, never::Never};
use tokio::{net::TcpStream, sync::broadcast};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{info, trace};
use wordbase::protocol::HookSentence;

use crate::{Config, Event};

pub async fn run(config: Arc<Config>, send_event: broadcast::Sender<Event>) -> Result<Never> {
    loop {
        tokio::time::sleep(config.textractor_connect_interval).await;

        let (stream, _) = match tokio_tungstenite::connect_async(&config.textractor_url).await {
            Ok(stream) => stream,
            Err(err) => {
                trace!("Failed to connect to Textractor: {err:?}");
                continue;
            }
        };

        info!("Textractor connected");
        let Err(err) = handle_stream(stream, send_event.clone()).await;
        info!("Textractor disconnected: {err:?}");
    }
}

async fn handle_stream(
    mut stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    send_event: broadcast::Sender<Event>,
) -> Result<Never> {
    loop {
        let message = stream
            .next()
            .await
            .context("stream closed")?
            .context("stream error")?
            .into_text()
            .context("received message which is not UTF-8 text")?;
        let sentence = serde_json::from_str::<HookSentence>(&message)
            .context("received message which is not a texthooker sentence")?;
        _ = send_event.send(Event::HookSentence(sentence));
    }
}
