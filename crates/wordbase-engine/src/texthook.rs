use std::time::Duration;

use anyhow::{Context, Result};
use futures::{StreamExt, never::Never};
use tokio::{
    net::TcpStream,
    sync::{broadcast, mpsc},
    time,
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
use tracing::{info, trace};
use wordbase::hook::HookSentence;

use crate::{CHANNEL_BUF_CAP, Engine, Event};

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
        self.pull_texthooker.send_new_url.send(url).await?;
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct PullTexthooker {
    send_new_url: mpsc::Sender<String>,
}

impl PullTexthooker {
    pub fn new(
        send_event: broadcast::Sender<Event>,
    ) -> (Self, impl Future<Output = Result<Never>>) {
        let (send_new_url, recv_new_url) = mpsc::channel(CHANNEL_BUF_CAP);
        (Self { send_new_url }, run(send_event, recv_new_url))
    }
}

async fn run(
    send_event: broadcast::Sender<Event>,
    mut recv_new_url: mpsc::Receiver<String>,
) -> Result<Never> {
    let mut current_url_task = None;
    loop {
        let new_url = recv_new_url.recv().await.context("channel closed")?;

        let new_task = tokio::spawn(handle_url(send_event.clone(), new_url));
        if let Some(old_task) = current_url_task.replace(new_task) {
            old_task.abort();
            _ = send_event.send(Event::PullTexthookerDisconnected);
        }
    }
}

async fn handle_url(send_event: broadcast::Sender<Event>, url: String) -> ! {
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
        _ = send_event.send(Event::PullTexthookerConnected);
        let Err(err) = handle_stream(&send_event, stream).await;
        info!("Disconnected: {err:?}");
        _ = send_event.send(Event::PullTexthookerDisconnected);
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
            .context("channel closed")?
            .context("connection error")?
            .into_data();
        let sentence = serde_json::from_slice::<HookSentence>(&message)
            .context("failed to deserialize message as hook sentence")?;
        _ = send_event.send(Event::HookSentence(sentence));
    }
}
