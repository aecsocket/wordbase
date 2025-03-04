use std::sync::Arc;

use anyhow::{Context, Result};
use futures::{StreamExt, never::Never};
use tokio::{net::TcpStream, sync::broadcast};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::info;
use wordbase::protocol::NewSentence;

use crate::Config;

pub async fn run(
    config: Arc<Config>,
    send_new_sentence: broadcast::Sender<NewSentence>,
) -> Result<Never> {
    loop {
        tokio::time::sleep(config.textractor_connect_interval).await;

        let (stream, _) = match tokio_tungstenite::connect_async(&config.textractor_url).await {
            Ok(stream) => stream,
            Err(err) => {
                info!("err: {err:?}");
                continue;
            }
        };

        info!("Textractor connected");
        let Err(err) = handle_stream(stream, send_new_sentence.clone()).await;
        info!("Textractor disconnected: {err:?}");
    }
}

async fn handle_stream(
    mut stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    send_new_sentence: broadcast::Sender<NewSentence>,
) -> Result<Never> {
    loop {
        let message = stream
            .next()
            .await
            .context("stream closed")?
            .context("stream error")?
            .into_text()
            .context("received message which is not UTF-8 text")?;
        let new_sentence = serde_json::from_str::<NewSentence>(&message)
            .context("received message which is not a `NewSentence`")?;
        _ = send_new_sentence.send(new_sentence);
    }
}
