use {
    crate::{Event, TexthookerSource},
    anyhow::{Context, Result},
    futures::{StreamExt, never::Never},
    std::sync::Arc,
    tokio::{net::TcpStream, sync::broadcast},
    tokio_tungstenite::{MaybeTlsStream, WebSocketStream},
    tracing::{info, trace},
    wordbase::hook::HookSentence,
};

pub async fn run(
    config: Arc<TexthookerSource>,
    send_event: broadcast::Sender<Event>,
) -> Result<Never> {
    loop {
        tokio::time::sleep(config.connect_interval).await;

        let (stream, _) = match tokio_tungstenite::connect_async(&config.url).await {
            Ok(stream) => stream,
            Err(err) => {
                trace!("Failed to connect: {err:?}");
                continue;
            }
        };

        info!("Connected");
        let Err(err) = handle_stream(stream, send_event.clone()).await;
        info!("Disconnected: {err:?}");
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
