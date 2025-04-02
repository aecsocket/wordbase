use {
    crate::{CHANNEL_BUF_CAP, Engine},
    anyhow::{Context, Result},
    futures::{StreamExt, never::Never},
    std::time::Duration,
    tokio::{
        net::TcpStream,
        sync::{broadcast, mpsc},
        time,
    },
    tokio_tungstenite::{MaybeTlsStream, WebSocketStream},
    tracing::{info, trace},
    wordbase::TexthookerSentence,
};

impl Engine {
    pub async fn texthooker_url(&self) -> Result<String> {
        let record = sqlx::query!("SELECT texthooker_url FROM config")
            .fetch_one(&self.db)
            .await?;
        Ok(record.texthooker_url)
    }

    pub async fn set_texthooker_url(&self, url: impl Into<String>) -> Result<()> {
        let url = url.into();
        sqlx::query!("UPDATE config SET texthooker_url = $1", url)
            .execute(&self.db)
            .await?;
        _ = self.texthookers.send_new_url.send(url);
        Ok(())
    }

    pub async fn texthooker_task(
        &self,
    ) -> Result<(
        impl Future<Output = Result<Never>> + use<>,
        mpsc::Receiver<TexthookerEvent>,
    )> {
        let initial_url = self
            .texthooker_url()
            .await
            .context("failed to fetch initial texthooker URL")?;
        let (send_event, recv_event) = mpsc::channel(CHANNEL_BUF_CAP);
        let recv_new_url = self.texthookers.send_new_url.subscribe();
        Ok((run(initial_url, send_event, recv_new_url), recv_event))
    }
}

#[derive(Debug)]
pub enum TexthookerEvent {
    Connected,
    Disconnected { reason: anyhow::Error },
    Replaced,
    Sentence(TexthookerSentence),
}

#[derive(Debug)]
pub(super) struct Texthookers {
    send_new_url: broadcast::Sender<String>,
}

impl Texthookers {
    pub fn new() -> Self {
        let (send_new_url, _) = broadcast::channel(CHANNEL_BUF_CAP);
        Self { send_new_url }
    }
}

async fn run(
    initial_url: String,
    send_event: mpsc::Sender<TexthookerEvent>,
    mut recv_new_url: broadcast::Receiver<String>,
) -> Result<Never> {
    let mut current_task = tokio::spawn(handle_url(send_event.clone(), initial_url));
    loop {
        let new_url = recv_new_url
            .recv()
            .await
            .context("new URL channel closed")?;

        current_task.abort();
        send_event.send(TexthookerEvent::Replaced).await?;
        current_task = tokio::spawn(handle_url(send_event.clone(), new_url));
    }
}

async fn handle_url(send_event: mpsc::Sender<TexthookerEvent>, url: String) -> Result<Never> {
    const RECONNECT_INTERVAL: Duration = Duration::from_secs(1);

    if url.trim().is_empty() {
        info!("Stopped attempting to connect");
        std::future::pending::<()>().await;
    }

    info!("Connecting to {url:?}");
    loop {
        let stream = match tokio_tungstenite::connect_async(&url).await {
            Ok((stream, _)) => stream,
            Err(err) => {
                trace!("Failed to connect: {err:?}");
                time::sleep(RECONNECT_INTERVAL).await;
                continue;
            }
        };

        info!("Connected to {url:?}");
        send_event.send(TexthookerEvent::Connected).await?;
        let Err(reason) = handle_stream(&send_event, stream).await;
        info!("Disconnected: {reason:?}");
        send_event
            .send(TexthookerEvent::Disconnected { reason })
            .await?;
    }
}

async fn handle_stream(
    send_event: &mpsc::Sender<TexthookerEvent>,
    mut stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Result<Never> {
    loop {
        let message = stream
            .next()
            .await
            .context("channel closed")?
            .context("connection error")?
            .into_data();
        let sentence = serde_json::from_slice::<TexthookerSentence>(&message)
            .context("failed to deserialize message as hook sentence")?;
        send_event.send(TexthookerEvent::Sentence(sentence)).await?;
    }
}
