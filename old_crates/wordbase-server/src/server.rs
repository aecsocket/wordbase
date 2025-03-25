use {
    crate::{
        Config, Event, db,
        import::{self, ImportError, Tracker},
        lookup, popup,
    },
    anyhow::{Context as _, Result},
    derive_more::{Deref, DerefMut},
    futures::{SinkExt as _, StreamExt as _, never::Never},
    sqlx::{Pool, Sqlite},
    std::{num::Wrapping, sync::Arc},
    tokio::{
        net::{TcpListener, TcpStream},
        sync::{Semaphore, broadcast, oneshot},
        task::JoinSet,
    },
    tokio_tungstenite::{WebSocketStream, tungstenite::Message},
    tracing::{Instrument, debug, info, info_span, warn},
    wordbase::protocol::{FromClient, FromServer},
};

#[derive(Debug, Clone)]
pub struct State {
    pub config: Arc<Config>,
    pub db: Pool<Sqlite>,
    pub lookups: lookup::Client,
    pub popups: popup::Client,
    pub send_event: broadcast::Sender<Event>,
    pub concurrent_imports: Arc<Semaphore>,
}

pub async fn run(state: State) -> Result<Never> {
    send_dictionary_sync(&state.db, &state.send_event)
        .await
        .context("failed to sync initial dictionaries")?;

    let listener = TcpListener::bind(&state.config.listen_addr)
        .await
        .context("failed to bind TCP listener")?;
    info!("Listening on {:?}", state.config.listen_addr);

    let mut connection_id = Wrapping(0usize);
    loop {
        let (stream, peer_addr) = listener
            .accept()
            .await
            .context("failed to accept TCP stream")?;

        let state = state.clone();
        tokio::spawn(
            async move {
                info!("Incoming connection from {peer_addr:?}");
                let Err(err) = handle_stream(state, stream).await;
                info!("Connection lost: {err:?}");
            }
            .instrument(info_span!("connection", id = %connection_id)),
        );
        connection_id += 1;
    }
}

#[derive(Debug, Deref, DerefMut)]
struct Connection(WebSocketStream<TcpStream>);

impl Connection {
    async fn write(&mut self, message: &FromServer) -> Result<()> {
        let message = serde_json::to_string(message).context("failed to serialize message")?;
        self.send(Message::text(message)).await?;
        Ok(())
    }
}

async fn handle_stream(state: State, stream: TcpStream) -> Result<Never> {
    let stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to accept WebSocket stream")?;
    let mut connection = Connection(stream);
    let mut recv_event = state.send_event.subscribe();

    let dictionaries = db::dictionary::all(&state.db)
        .await
        .context("failed to fetch dictionaries")?;

    connection
        .write(&FromServer::SyncLookupConfig {
            lookup_config: state.config.lookup.clone(),
        })
        .await
        .context("failed to sync lookup config")?;
    connection
        .write(&FromServer::SyncDictionaries { dictionaries })
        .await
        .context("failed to sync dictionaries")?;

    loop {
        tokio::select! {
            Ok(event) = recv_event.recv() => {
                forward_event(&mut connection, event).await;
            }
            data = connection.next() => {
                let data = data
                    .context("stream closed")?
                    .context("stream error")?;
                if let Err(err) = handle_message(
                    &state,
                    &mut connection,
                    data,
                )
                .await
                {
                    warn!("Failed to handle request: {err:?}");
                    let message = format!("{err:?}");
                    _ = connection.write(&FromServer::Error { message }).await;
                }
            }
        }
    }
}

async fn forward_event(connection: &mut Connection, event: Event) {
    let message = match event {
        Event::HookSentence(sentencece) => FromServer::HookSentence(sentencece),
        Event::SyncDictionaries(dictionaries) => FromServer::SyncDictionaries { dictionaries },
    };

    _ = connection.write(&message).await;
}

async fn send_dictionary_sync(
    db: &Pool<Sqlite>,
    send_event: &broadcast::Sender<Event>,
) -> Result<()> {
    let dictionaries = db::dictionary::all(db)
        .await
        .context("failed to fetch dictionaries")?;

    _ = send_event.send(Event::SyncDictionaries(dictionaries));
    Ok(())
}

async fn handle_message(state: &State, connection: &mut Connection, data: Message) -> Result<()> {
    let data = data.into_data();
    if data.is_empty() {
        return Ok(());
    }
    let message =
        serde_json::from_slice::<FromClient>(&data).context("received invalid message")?;
    debug!("{message:#?}");

    match message {
        FromClient::HookSentence(sentence) => {
            debug!("{sentence:#?}");
            _ = state.send_event.send(Event::HookSentence(sentence));
            Ok(())
        }
        FromClient::Lookup(request) => {
            let records = state
                .lookups
                .lookup(request)
                .await
                .context("failed to perform lookup")?;
            for record in records {
                connection
                    .write(&FromServer::Lookup(record))
                    .await
                    .context("failed to send record")?;
            }
            connection
                .write(&FromServer::LookupDone)
                .await
                .context("failed to send lookup done")?;
            Ok(())
        }
        FromClient::ShowPopup(request) => {
            let result = state
                .popups
                .show(request)
                .await
                .context("failed to show popup")?;
            connection
                .write(&FromServer::ShowPopup { result })
                .await
                .context("failed to send show popup response")?;
            Ok(())
        }
        FromClient::HidePopup => {
            state.popups.hide().await.context("failed to hide popup")?;
            connection
                .write(&FromServer::HidePopup)
                .await
                .context("failed to send hide popup response")?;
            Ok(())
        }
    }
}
