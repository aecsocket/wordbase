use {
    crate::{CHANNEL_BUF_CAP, Engine, EngineEvent},
    anyhow::{Context, Result},
    arc_swap::ArcSwap,
    futures::{StreamExt, never::Never},
    sqlx::{Pool, Sqlite, query},
    std::{
        sync::{
            Arc,
            atomic::{self, AtomicU8},
        },
        time::Duration,
    },
    tokio::{
        net::TcpStream,
        sync::{broadcast, mpsc},
        time,
    },
    tokio_tungstenite::{MaybeTlsStream, WebSocketStream},
    tokio_util::task::AbortOnDropHandle,
    tracing::{debug, info, trace},
    wordbase::TexthookerSentence,
};

#[derive(Debug)]
pub struct Texthookers {
    url: Arc<ArcSwap<String>>,
    connected: Arc<AtomicU8>,
    send_new_url: mpsc::Sender<()>,
    _task: AbortOnDropHandle<Never>,
}

impl Texthookers {
    pub(super) async fn new(
        db: &Pool<Sqlite>,
        send_event: broadcast::Sender<EngineEvent>,
    ) -> Result<Self> {
        let url = query!("SELECT texthooker_url FROM config")
            .fetch_one(db)
            .await
            .context("failed to fetch initial URL")?
            .texthooker_url;
        let url = Arc::new(ArcSwap::from_pointee(url));
        let connected = Arc::new(AtomicU8::new(0));

        let (send_new_url, recv_new_url) = mpsc::channel(CHANNEL_BUF_CAP);
        let task = AbortOnDropHandle::new(tokio::spawn(run(
            send_event,
            url.clone(),
            connected.clone(),
            recv_new_url,
        )));
        Ok(Self {
            url,
            connected,
            send_new_url,
            _task: task,
        })
    }
}

impl Engine {
    #[must_use]
    pub fn texthooker_url(&self) -> Arc<String> {
        self.texthookers.url.load().clone()
    }

    #[must_use]
    pub fn texthooker_connected(&self) -> bool {
        self.texthookers.connected.load(atomic::Ordering::SeqCst) == 1
    }

    pub async fn set_texthooker_url(&self, url: impl Into<String>) -> Result<()> {
        let url = url.into();
        query!("UPDATE config SET texthooker_url = $1", url)
            .execute(&self.db)
            .await?;
        self.texthookers.url.store(Arc::new(url));
        _ = self.texthookers.send_new_url.send(()).await;
        Ok(())
    }
}

pub(super) async fn run(
    send_event: broadcast::Sender<EngineEvent>,
    url: Arc<ArcSwap<String>>,
    connected: Arc<AtomicU8>,
    mut recv_new_url: mpsc::Receiver<()>,
) -> Never {
    let mut current_task = handle_url(&send_event, url.load().clone(), connected.clone());
    loop {
        tokio::select! {
            _ = current_task => {},
            Some(()) = recv_new_url.recv() => {},
        };

        connected.store(0, atomic::Ordering::SeqCst);
        _ = send_event.send(EngineEvent::PullTexthookerDisconnected);
        current_task = handle_url(&send_event, url.load().clone(), connected.clone());
    }
}

async fn handle_url(
    send_event: &broadcast::Sender<EngineEvent>,
    url: Arc<String>,
    connected: Arc<AtomicU8>,
) -> ! {
    const RECONNECT_INTERVAL: Duration = Duration::from_secs(1);

    if url.trim().is_empty() {
        info!("Stopped attempting to connect");
        std::future::pending::<()>().await;
    }

    debug!("Connecting to {url:?}");
    loop {
        let stream = match tokio_tungstenite::connect_async(&*url).await {
            Ok((stream, _)) => stream,
            Err(err) => {
                trace!("Failed to connect: {err:?}");
                time::sleep(RECONNECT_INTERVAL).await;
                continue;
            }
        };

        info!("Connected to {url:?}");
        connected.store(1, atomic::Ordering::SeqCst);
        _ = send_event.send(EngineEvent::PullTexthookerConnected);

        let Err(err) = handle_stream(send_event, stream).await;

        info!("Disconnected: {err:?}");
        connected.store(0, atomic::Ordering::SeqCst);
        _ = send_event.send(EngineEvent::PullTexthookerDisconnected);
    }
}

async fn handle_stream(
    send_event: &broadcast::Sender<EngineEvent>,
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
        _ = send_event.send(EngineEvent::TexthookerSentence(sentence));
    }
}
