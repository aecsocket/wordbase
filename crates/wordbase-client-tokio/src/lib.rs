#![doc = include_str!("../README.md")]

pub use {indexmap, wordbase};

use std::pin::Pin;

use derive_more::{Display, Error};
use futures::{Sink, SinkExt, Stream, StreamExt, task};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
use wordbase::{
    Dictionary, DictionaryId, LookupConfig,
    hook::HookSentence,
    protocol::{DictionaryNotFound, FromClient, FromServer, RecordLookup},
};

#[derive(Debug, Clone, Copy)]
pub struct Connection<S>(pub S);

impl<S> Connection<S>
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
{
    pub async fn recv(&mut self) -> Result<FromServer, ConnectionError> {
        let data = self
            .0
            .next()
            .await
            .ok_or(ConnectionError::StreamClosed)?
            .map_err(ConnectionError::Stream)?
            .into_data();
        serde_json::from_slice::<FromServer>(&data).map_err(ConnectionError::Deserialize)
    }

    pub async fn send(&mut self, message: &FromClient) -> Result<(), ConnectionError> {
        let request = serde_json::to_string(message).map_err(ConnectionError::Serialize)?;
        self.0
            .send(Message::text(request))
            .await
            .map_err(ConnectionError::Send)
    }
}

/// WebSocket error type.
pub type WsError = tokio_tungstenite::tungstenite::Error;

#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum ConnectionError {
    /// Failed to connect to server.
    #[display("failed to connect")]
    Connect(WsError),
    /// Failed to complete handshaking with server.
    #[display("failed to handshake")]
    Handshake(Box<ConnectionError>),
    /// Failed to serialize request.
    #[display("failed to serialize request")]
    Serialize(serde_json::Error),
    /// Failed to send request to the server.
    #[display("failed to send request")]
    Send(WsError),
    /// WebSocket stream is closed.
    #[display("stream closed")]
    StreamClosed,
    /// WebSocket stream error.
    #[display("stream error")]
    Stream(WsError),
    /// Failed to deserialize server response.
    #[display("failed to deserialize response")]
    Deserialize(serde_json::Error),
    /// Received a valid message from the server, but we expected a different
    /// kind of message.
    #[display("received wrong message kind")]
    WrongMessageKind,
    /// Server is sending us a custom error.
    Server(#[error(not(source))] String),
}

#[derive(Debug)]
pub struct Client<S> {
    connection: Connection<S>,
    lookup_config: LookupConfig,
    dictionaries: IndexMap<DictionaryId, Dictionary>,
    events: Vec<Event>,
}

pub type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

pub type SocketClient = Client<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Connects to a Wordbase server at the given URL and sets up a [`Client`].
///
/// See [`tokio_tungstenite::connect_async`] and [`Client::handshake`].
///
/// # Errors
///
/// See [`ConnectionError`].
pub async fn connect(
    request: impl IntoClientRequest + Unpin,
) -> Result<SocketClient, ConnectionError> {
    let (stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(ConnectionError::Connect)?;
    Client::handshake(stream)
        .await
        .map_err(|(err, _)| ConnectionError::Handshake(Box::new(err)))
}

#[derive(Debug)]
pub enum Event {
    SyncLookupConfig,
    SyncDictionaries,
    HookSentence(HookSentence),
}

impl<S> Client<S>
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
{
    pub async fn handshake(stream: S) -> Result<Self, (ConnectionError, S)> {
        let mut connection = Connection(stream);

        let lookup_config = match connection.recv().await {
            Ok(FromServer::SyncLookupConfig { lookup_config }) => lookup_config,
            Ok(_) => {
                return Err((ConnectionError::WrongMessageKind, connection.0));
            }
            Err(err) => {
                return Err((err, connection.0));
            }
        };

        let dictionaries = match connection.recv().await {
            Ok(FromServer::SyncDictionaries { dictionaries }) => dictionaries,
            Ok(_) => {
                return Err((ConnectionError::WrongMessageKind, connection.0));
            }
            Err(err) => {
                return Err((err, connection.0));
            }
        };

        Ok(Self {
            connection,
            lookup_config,
            dictionaries: dictionaries
                .into_iter()
                .map(|dict| (dict.id, dict))
                .collect(),
            events: Vec::new(),
        })
    }

    #[must_use]
    pub const fn lookup_config(&self) -> &LookupConfig {
        &self.lookup_config
    }

    // doc: guaranteed to be ordered by dict order
    #[must_use]
    pub const fn dictionaries(&self) -> &IndexMap<DictionaryId, Dictionary> {
        &self.dictionaries
    }

    fn event_from(&mut self, message: FromServer) -> Result<Event, ConnectionError> {
        match message {
            FromServer::Error { message } => Err(ConnectionError::Server(message)),
            FromServer::SyncLookupConfig { lookup_config } => {
                self.lookup_config = lookup_config;
                Ok(Event::SyncLookupConfig)
            }
            FromServer::SyncDictionaries { dictionaries } => {
                self.dictionaries = dictionaries
                    .into_iter()
                    .map(|dict| (dict.id, dict))
                    .collect();
                Ok(Event::SyncDictionaries)
            }
            FromServer::HookSentence(sentence) => Ok(Event::HookSentence(sentence)),
            _ => Err(ConnectionError::WrongMessageKind),
        }
    }

    pub async fn poll(&mut self) -> Result<Event, ConnectionError> {
        if let Some(event) = self.events.pop() {
            return Ok(event);
        }

        let message = self.connection.recv().await?;
        self.event_from(message)
    }

    fn fallback_handle(&mut self, message: FromServer) -> Result<(), ConnectionError> {
        let event = self.event_from(message)?;
        self.events.push(event);
        Ok(())
    }

    pub async fn hook_sentence(&mut self, sentence: HookSentence) -> Result<(), ConnectionError> {
        self.connection
            .send(&FromClient::HookSentence(sentence))
            .await?;
        Ok(())
    }

    pub async fn lookup(
        &mut self,
        text: impl Into<String>,
    ) -> Result<Pin<Box<impl Stream<Item = Result<RecordLookup, ConnectionError>>>>, ConnectionError>
    {
        self.connection
            .send(&FromClient::Lookup { text: text.into() })
            .await?;

        // TODO: actual stream
        let mut all_records = Vec::<RecordLookup>::new();
        loop {
            match self.connection.recv().await? {
                FromServer::Lookup { record } => {
                    all_records.push(record);
                }
                FromServer::LookupDone => break,
                message => self.fallback_handle(message)?,
            }
        }

        Ok(Box::pin(futures::stream::iter(
            all_records.into_iter().map(Ok),
        )))
    }

    pub async fn remove_dictionary(
        &mut self,
        dictionary_id: DictionaryId,
    ) -> Result<Result<(), DictionaryNotFound>, ConnectionError> {
        self.connection
            .send(&FromClient::RemoveDictionary { dictionary_id })
            .await?;
        loop {
            match self.connection.recv().await? {
                FromServer::RemoveDictionary { result } => return Ok(result),
                message => self.fallback_handle(message)?,
            }
        }
    }

    pub async fn set_dictionary_enabled(
        &mut self,
        dictionary_id: DictionaryId,
        enabled: bool,
    ) -> Result<Result<(), DictionaryNotFound>, ConnectionError> {
        self.connection
            .send(&FromClient::SetDictionaryEnabled {
                dictionary_id,
                enabled,
            })
            .await?;
        loop {
            match self.connection.recv().await? {
                FromServer::SetDictionaryEnabled { result } => return Ok(result),
                message => self.fallback_handle(message)?,
            }
        }
    }

    pub async fn enable_dictionary(
        &mut self,
        dictionary_id: DictionaryId,
    ) -> Result<Result<(), DictionaryNotFound>, ConnectionError> {
        self.set_dictionary_enabled(dictionary_id, true).await
    }

    pub async fn disable_dictionary(
        &mut self,
        dictionary_id: DictionaryId,
    ) -> Result<Result<(), DictionaryNotFound>, ConnectionError> {
        self.set_dictionary_enabled(dictionary_id, false).await
    }
}

impl<S> Client<WebSocketStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    pub async fn close(&mut self) -> Result<(), ConnectionError> {
        self.connection
            .0
            .close(None)
            .await
            .map_err(ConnectionError::Stream)
    }
}

struct Lookup<'c, S> {
    client: &'c mut Client<S>,
}

impl<S> Stream for Lookup<'_, S>
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
{
    type Item = Result<RecordLookup, ConnectionError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        todo!()
    }
}
