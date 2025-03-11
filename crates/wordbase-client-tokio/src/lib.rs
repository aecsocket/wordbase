#![doc = include_str!("../README.md")]

use {
    derive_more::{Display, Error},
    futures::{Sink, SinkExt, Stream, StreamExt},
    std::pin::Pin,
    tokio::{
        io::{AsyncRead, AsyncWrite},
        net::TcpStream,
    },
    tokio_tungstenite::{
        MaybeTlsStream, WebSocketStream,
        tungstenite::{Message, client::IntoClientRequest},
    },
    wordbase::{
        DictionaryId, DictionaryState,
        hook::HookSentence,
        protocol::{
            DictionaryNotFound, FromClient, FromServer, LookupConfig, LookupRequest,
            LookupResponse, NoRecords, ShowPopupRequest, ShowPopupResponse,
        },
    },
};
pub use {indexmap, wordbase};

/// Raw connection to a Wordbase server.
///
/// This does not handle any state, just sending and receiving messages. Prefer
/// using [`Client`] for a more high-level API.
#[derive(Debug, Clone, Copy)]
pub struct Connection<S>(pub S);

impl<S> Connection<S>
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
{
    /// Receives a message from the server.
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
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

    /// Sends a message to the server.
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
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

/// Ways in which sending or receiving a Wordbase message may fail.
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

/// High-level API for interfacing with a Wordbase server's WebSocket API.
///
/// Use [`connect`] to create one.
#[derive(Debug)]
pub struct Client<S> {
    connection: Connection<S>,
    lookup_config: LookupConfig,
    dictionaries: IndexMap<DictionaryId, DictionaryState>,
    events: Vec<Event>,
}

/// [`indexmap::IndexMap`] using the [`foldhash::fast::RandomState`] hasher.
pub type IndexMap<K, V> = indexmap::IndexMap<K, V, foldhash::fast::RandomState>;

/// [`Client`] which wraps a [`WebSocketStream`].
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

/// Event which may have been sent by the server to us while we were busy with
/// another task.
#[derive(Debug)]
pub enum Event {
    /// Server updated its lookup config.
    SyncLookupConfig,
    /// Server updated its dictionary set.
    SyncDictionaries,
    /// Server forwarded us a new [`HookSentence`] from another client.
    HookSentence(HookSentence),
}

impl<S> Client<S>
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
{
    /// Performs a handshake with the server using an existing stream.
    ///
    /// This will sync initial data with the server, and set up a [`Client`] for
    /// future use.
    ///
    /// # Errors
    ///
    /// Errors if there was a connection or handshaking error, i.e. the server
    /// sent us the wrong kind of message at the wrong time. The error will
    /// also return ownership of the stream.
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

    /// Gets a shared reference to the last lookup config that the server synced
    /// to us.
    #[must_use]
    pub const fn lookup_config(&self) -> &LookupConfig {
        &self.lookup_config
    }

    /// Gets a shared reference to the last dictionary set that the server
    /// synced to us.
    ///
    /// These dictionaries are guaranteed to be sorted by
    /// [`DictionaryState::position`].
    #[must_use]
    pub const fn dictionaries(&self) -> &IndexMap<DictionaryId, DictionaryState> {
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

    /// Consumes an [`Event`] if the server sent us an event while we were busy
    /// with another task.
    ///
    /// See [`poll`] for an async version.
    pub fn try_poll(&mut self) -> Option<Event> {
        self.events.pop()
    }

    /// Consumes an [`Event`] if the server sent us an event while we were busy
    /// with another task, or waits for the server to send us one.
    ///
    /// See [`try_poll`] for a sync, non-blocking version.
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
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

    /// Sends a [`HookSentence`].
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
    pub async fn hook_sentence(&mut self, sentence: HookSentence) -> Result<(), ConnectionError> {
        self.connection
            .send(&FromClient::HookSentence(sentence))
            .await?;
        Ok(())
    }

    /// Performs a [`LookupRequest`], returning the [`LookupResponse`]s.
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
    pub async fn lookup(
        &mut self,
        request: LookupRequest,
    ) -> Result<
        Pin<Box<impl Stream<Item = Result<LookupResponse, ConnectionError>>>>,
        ConnectionError,
    > {
        self.connection.send(&FromClient::from(request)).await?;

        // TODO: we need async iterators!
        let mut all_responses = Vec::<LookupResponse>::new();
        loop {
            match self.connection.recv().await? {
                FromServer::Lookup(response) => {
                    all_responses.push(response);
                }
                FromServer::LookupDone => break,
                message => self.fallback_handle(message)?,
            }
        }

        Ok(Box::pin(futures::stream::iter(
            all_responses.into_iter().map(Ok),
        )))
    }

    /// Sends a [`ShowPopupRequest`].
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
    pub async fn show_popup(
        &mut self,
        request: ShowPopupRequest,
    ) -> Result<Result<ShowPopupResponse, NoRecords>, ConnectionError> {
        self.connection.send(&FromClient::from(request)).await?;
        loop {
            match self.connection.recv().await? {
                FromServer::ShowPopup { result } => return Ok(result),
                message => self.fallback_handle(message)?,
            }
        }
    }

    /// Sends a [`HidePopupRequest`].
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
    pub async fn hide_popup(&mut self) -> Result<(), ConnectionError> {
        self.connection.send(&FromClient::HidePopup).await?;
        loop {
            match self.connection.recv().await? {
                FromServer::HidePopup => return Ok(()),
                message => self.fallback_handle(message)?,
            }
        }
    }

    /// Sends a [`FromClient::RemoveDictionary`].
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
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

    /// Sends a [`FromClient::SetDictionaryEnabled`].
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
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

    /// Sends a [`FromClient::SetDictionaryEnabled`] enabling a dictionary.
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
    pub async fn enable_dictionary(
        &mut self,
        dictionary_id: DictionaryId,
    ) -> Result<Result<(), DictionaryNotFound>, ConnectionError> {
        self.set_dictionary_enabled(dictionary_id, true).await
    }

    /// Sends a [`FromClient::SetDictionaryEnabled`] disabling a dictionary.
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
    pub async fn disable_dictionary(
        &mut self,
        dictionary_id: DictionaryId,
    ) -> Result<Result<(), DictionaryNotFound>, ConnectionError> {
        self.set_dictionary_enabled(dictionary_id, false).await
    }

    /// Sends a [`FromClient::SetDictionaryPosition`].
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
    pub async fn set_dictionary_position(
        &mut self,
        dictionary_id: DictionaryId,
        position: i64,
    ) -> Result<Result<(), DictionaryNotFound>, ConnectionError> {
        self.connection
            .send(&FromClient::SetDictionaryPosition {
                dictionary_id,
                position,
            })
            .await?;
        loop {
            match self.connection.recv().await? {
                FromServer::SetDictionaryPosition { result } => return Ok(result),
                message => self.fallback_handle(message)?,
            }
        }
    }
}

impl<S> Client<WebSocketStream<S>>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    /// Closes the underlying [`WebSocketStream`].
    ///
    /// # Errors
    ///
    /// See [`ConnectionError`].
    pub async fn close(&mut self) -> Result<(), ConnectionError> {
        self.connection
            .0
            .close(None)
            .await
            .map_err(ConnectionError::Stream)
    }
}
