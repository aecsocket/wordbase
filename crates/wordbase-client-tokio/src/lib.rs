#![doc = include_str!("../README.md")]

use derive_more::{Display, Error};
pub use wordbase;

use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
use wordbase::{
    SharedConfig,
    dict::{Dictionary, DictionaryId},
    protocol::{DictionaryNotFound, FromClient, FromServer, LookupInfo, NewSentence},
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
    config: SharedConfig,
    events: Vec<Event>,
}

pub type SocketClient = Client<WebSocketStream<MaybeTlsStream<TcpStream>>>;

#[derive(Debug)]
pub enum Event {
    NewConfig,
    NewSentence(NewSentence),
}

impl<S> Client<S>
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
{
    pub async fn handshake(stream: S) -> Result<Self, (ConnectionError, S)> {
        let mut connection = Connection(stream);
        let config = match connection.recv().await {
            Ok(FromServer::SyncConfig { config }) => config,
            Ok(_) => {
                return Err((ConnectionError::WrongMessageKind, connection.0));
            }
            Err(err) => {
                return Err((err, connection.0));
            }
        };

        Ok(Self {
            connection,
            config,
            events: Vec::new(),
        })
    }

    fn event_from(&mut self, message: FromServer) -> Result<Event, ConnectionError> {
        match message {
            FromServer::Error { message } => Err(ConnectionError::Server(message)),
            FromServer::SyncConfig { config } => {
                self.config = config;
                Ok(Event::NewConfig)
            }
            FromServer::NewSentence(new_sentence) => Ok(Event::NewSentence(new_sentence)),
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

    pub async fn new_sentence(&mut self, new_sentence: NewSentence) -> Result<(), ConnectionError> {
        self.connection
            .send(&FromClient::NewSentence(new_sentence))
            .await?;
        Ok(())
    }

    pub async fn lookup(
        &mut self,
        text: impl Into<String>,
    ) -> Result<Option<LookupInfo>, ConnectionError> {
        self.connection
            .send(&FromClient::Lookup { text: text.into() })
            .await?;
        loop {
            match self.connection.recv().await? {
                FromServer::Lookup { lookup } => return Ok(lookup),
                message => self.fallback_handle(message)?,
            }
        }
    }

    pub async fn list_dictionaries(&mut self) -> Result<Vec<Dictionary>, ConnectionError> {
        self.connection.send(&FromClient::ListDictionaries).await?;
        loop {
            match self.connection.recv().await? {
                FromServer::ListDictionaries { dictionaries } => return Ok(dictionaries),
                message => self.fallback_handle(message)?,
            }
        }
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
    Client::handshake(stream).await.map_err(|(err, _)| err)
}
