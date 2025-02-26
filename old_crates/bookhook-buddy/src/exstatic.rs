use anyhow::{Context as _, Result};
use futures::{StreamExt as _, never::Never};
use serde::{Deserialize, Serialize};
use tokio::{sync::mpsc, task::JoinHandle};
use tracing::{Instrument, info, info_span, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewSentence {
    pub process_path: String,
    pub sentence: String,
}

pub async fn run(
    mut recv_server_url: mpsc::Receiver<String>,
    send_new_sentence: mpsc::Sender<NewSentence>,
) -> Result<Never> {
    let mut current_connection = None::<JoinHandle<()>>;
    loop {
        let server_url = recv_server_url
            .recv()
            .await
            .context("server URL channel closed")?;
        if let Some(current_connection) = current_connection.take() {
            current_connection.abort();
        }

        let span = info_span!("connect", %server_url);
        let send_new_sentence = send_new_sentence.clone();
        current_connection = Some(tokio::spawn(
            async move {
                info!("Connecting");
                let Err(err) = on_new_url(&server_url, send_new_sentence).await;
                warn!("Connection lost: {err:?}");
            }
            .instrument(span),
        ));
    }
}

// {"process_path":"foo","sentence":"hello world"}
async fn on_new_url(
    server_url: &str,
    send_new_sentence: mpsc::Sender<NewSentence>,
) -> Result<Never> {
    let (mut stream, _) = tokio_tungstenite::connect_async(server_url)
        .await
        .context("failed to connect to server")?;

    info!("Connected");
    loop {
        let message = stream
            .next()
            .await
            .context("stream closed")?
            .context("stream error")?
            .into_text()
            .context("received message which is not UTF-8 text")?;

        let new_sentence = serde_json::from_str::<NewSentence>(&message)
            .context("received message which is not a 'new sentence'")?;

        send_new_sentence
            .send(new_sentence)
            .await
            .context("new sentence channel closed")?;
    }
}
