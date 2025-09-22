use {
    eyre::{Context, OptionExt, Result},
    futures::{StreamExt, never::Never},
    std::{future, time::Duration},
    tokio::{net::TcpStream, time},
    tokio_tungstenite::{MaybeTlsStream, WebSocketStream},
    tokio_util::task::AbortOnDropHandle,
    tracing::{debug, info, trace},
    wordbase_types::TexthookerSentence,
};

/// Connects to a [`TextractorSender`] server and receives [sentences] from a
/// texthooker, forwarding them to your app.
///
/// When this is dropped, the backend task is aborted.
///
/// [`TextractorSender`]: https://github.com/KamWithK/TextractorSender
/// [sentences]: TexthookerSentence
#[derive(Debug)]
pub struct TexthookerClient {
    // use `async_channel` instead of `tokio::sync::mpsc`,
    // since it lets us `Sender::force_send`
    tx_url: async_channel::Sender<String>,
    _task: AbortOnDropHandle<Result<()>>,
}

/// [`TexthookerClient`] event.
#[derive(Debug, Clone)]
pub enum TexthookerEvent {
    /// Connected to a server.
    Connected,
    /// Disconnected from a server.
    Disconnected,
    /// Received a sentence from the connected server.
    Sentence(TexthookerSentence),
}

impl TexthookerClient {
    /// Creates a new client and spawns a task to handle the backend.
    ///
    /// # Panics
    ///
    /// Panics if run outside of a Tokio runtime.
    #[must_use]
    pub fn new() -> (Self, async_channel::Receiver<TexthookerEvent>) {
        let (tx_event, rx_event) = async_channel::bounded(4);
        let (tx_url, rx_url) = async_channel::bounded(1);
        let task = AbortOnDropHandle::new(tokio::spawn(run(tx_event, rx_url)));
        (
            Self {
                tx_url,
                _task: task,
            },
            rx_event,
        )
    }

    /// Sets the URL of the server to connect to.
    #[expect(
        clippy::missing_panics_doc,
        reason = "will only panic if the backend task is dropped, which would be a bug"
    )]
    pub fn set_url(&self, url: impl Into<String>) {
        // force send to discard any previous URLs;
        // the backend task will only try to connect to the latest URL given
        self.tx_url
            .force_send(url.into())
            .expect("backend task dropped");
    }
}

async fn run(
    tx_event: async_channel::Sender<TexthookerEvent>,
    rx_url: async_channel::Receiver<String>,
) -> Result<()> {
    let mut current_task = handle_url(&tx_event, String::new());
    loop {
        tokio::select! {
            _ = current_task => unreachable!("task must never return"),
            Err(_) = rx_url.recv() => return Ok(()),
            Ok(url) = rx_url.recv() => {
                current_task = handle_url(&tx_event, url);
            },
        };
        _ = tx_event.send(TexthookerEvent::Disconnected).await;
    }
}

async fn handle_url(tx_event: &async_channel::Sender<TexthookerEvent>, url: String) -> ! {
    const RECONNECT_INTERVAL: Duration = Duration::from_secs(1);

    let url = url.trim();
    if url.is_empty() {
        info!("URL is empty, will not connect");
        future::pending::<()>().await;
    }

    debug!("Connecting to {url:?}");
    loop {
        let stream = match tokio_tungstenite::connect_async(url).await {
            Ok((stream, _)) => stream,
            Err(err) => {
                trace!("Failed to connect: {err:?}");
                time::sleep(RECONNECT_INTERVAL).await;
                continue;
            }
        };

        info!("Connected to {url:?}");
        _ = tx_event.send(TexthookerEvent::Connected).await;

        let Err(err) = handle_stream(tx_event, stream).await;

        info!("Disconnected: {err:?}");
    }
}

async fn handle_stream(
    tx_event: &async_channel::Sender<TexthookerEvent>,
    mut stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
) -> Result<Never> {
    loop {
        let message = stream
            .next()
            .await
            .ok_or_eyre("channel closed")?
            .context("connection error")?
            .into_data();
        let sentence = serde_json::from_slice::<TexthookerSentence>(&message)
            .wrap_err("failed to deserialize message as hook sentence")?;
        _ = tx_event.send(TexthookerEvent::Sentence(sentence)).await;
    }
}
