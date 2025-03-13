use {
    crate::Event,
    anyhow::{Context, Result},
    futures::{StreamExt, never::Never},
    std::time::Duration,
    tokio::{net::TcpStream, sync::broadcast},
    tokio_tungstenite::{MaybeTlsStream, WebSocketStream},
    tracing::{debug, info, trace},
    wordbase::hook::HookSentence,
};

pub async fn run(
    source: &str,
    connect_interval: Duration,
    send_event: &broadcast::Sender<Event>,
) -> ! {
    loop {
        trace!("Attempting connection");
        let (stream, _) = match tokio_tungstenite::connect_async(source).await {
            Ok(stream) => stream,
            Err(err) => {
                trace!("Failed to connect: {err:?}");
                tokio::time::sleep(connect_interval).await;
                continue;
            }
        };

        info!("Connected");
        let Err(err) = handle_stream(send_event, stream).await;
        info!("Disconnected: {err:?}");
    }
}

async fn handle_stream(
    send_event: &broadcast::Sender<Event>,
    mut stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
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
        debug!("{sentence:#?}");
        _ = send_event.send(Event::HookSentence(sentence));
    }
}
