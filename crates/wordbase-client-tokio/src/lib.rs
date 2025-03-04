#![doc = include_str!("../README.md")]

use std::num::Wrapping;

use derive_more::{Display, Error};
pub use wordbase;

use futures::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
use wordbase::{
    lookup::{LookupInfo, SharedConfig},
    protocol::{ClientRequest, FromClient, FromServer, Lookup, NewSentence, RequestId, Response},
};

/// WebSocket error type.
pub type WsError = tokio_tungstenite::tungstenite::Error;

/// Error when using a [`Connection`].
#[derive(Debug, Display, Error)]
#[non_exhaustive]
pub enum Error {
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
    /// Failed to parse server response as UTF-8 text.
    #[display("failed to parse response as UTF-8 text")]
    ResponseIntoText(WsError),
    /// Failed to deserialize server response.
    #[display("failed to deserialize response")]
    Deserialize(serde_json::Error),
    /// Received invalid response kind from server.
    #[display("received invalid response kind")]
    InvalidResponseKind,
    /// Server sent us an error in response to one of our requests.
    FromServer(#[error(ignore)] String),
}

#[derive(Debug)]
pub struct Connection {
    pub stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
}

impl Connection {
    pub async fn recv(&mut self) -> Result<FromServer, Error> {
        let data = self
            .stream
            .next()
            .await
            .ok_or(Error::StreamClosed)?
            .map_err(Error::Stream)?
            .into_data();
        serde_json::from_slice::<FromServer>(&data).map_err(Error::Deserialize)
    }

    pub async fn send(&mut self, message: &FromClient) -> Result<(), Error> {
        let request = serde_json::to_string(message).map_err(Error::Serialize)?;
        self.stream
            .send(Message::text(request))
            .await
            .map_err(Error::Send)
    }
}

#[derive(Debug)]
pub struct Client {
    connection: Connection,
    config: SharedConfig,
    next_request_id: Wrapping<u64>,
    events: Vec<Event>,
}

#[derive(Debug)]
pub enum Event {
    SyncConfig,
    NewSentence(NewSentence),
    Response {
        request_id: RequestId,
        response: Response,
    },
}

impl Client {
    pub async fn handshake(mut connection: Connection) -> Result<Self, (Error, Connection)> {
        let lookup_config = match connection.recv().await {
            Ok(FromServer::SyncConfig(shared_config)) => shared_config,
            Ok(_) => {
                return Err((Error::InvalidResponseKind, connection));
            }
            Err(err) => {
                return Err((err, connection));
            }
        };
        Ok(Self {
            connection,
            config: lookup_config,
            next_request_id: Wrapping(0),
            events: Vec::new(),
        })
    }

    async fn send(&mut self, request: ClientRequest) -> Result<RequestId, Error> {
        let request_id = RequestId::from_raw(self.next_request_id.0);
        self.next_request_id += 1;
        self.connection
            .send(&FromClient {
                request_id,
                request,
            })
            .await?;
        Ok(request_id)
    }

    pub fn drain_events(&mut self) -> impl Iterator<Item = Event> {
        self.events.drain(..)
    }

    pub async fn poll(&mut self) -> Result<Event, Error> {
        if let Some(event) = self.events.pop() {
            return Ok(event);
        }

        let message = self.connection.recv().await?;
        self.handle(message)
    }

    fn handle(&mut self, message: FromServer) -> Result<Event, Error> {
        match message {
            FromServer::SyncConfig(config) => {
                self.config = config;
                Ok(Event::SyncConfig)
            }
            FromServer::NewSentence(new_sentence) => Ok(Event::NewSentence(new_sentence)),
            FromServer::Response {
                request_id,
                response,
            } => Ok(Event::Response {
                request_id,
                response,
            }),
            FromServer::Error(err) => Err(Error::FromServer(err)),
        }
    }

    pub async fn lookup(&mut self, request: Lookup) -> Result<Option<LookupInfo>, Error> {
        let this_request_id = self.send(ClientRequest::Lookup(request)).await?;
        loop {
            match self.connection.recv().await? {
                FromServer::Response {
                    request_id,
                    response: Response::LookupInfo(lookup_info),
                } if request_id == this_request_id => {
                    return Ok(lookup_info);
                }
                message => {
                    let event = self.handle(message)?;
                    self.events.push(event);
                }
            }
        }
    }
}

/// Connects to a Wordbase server at the given URL and sets up a [`Client`].
///
/// See [`tokio_tungstenite::connect_async`] and [`Client::handshake`].
///
/// # Errors
///
/// See [`WsError`].
pub async fn connect(request: impl IntoClientRequest + Unpin) -> Result<Client, Error> {
    let (stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(Error::Connect)?;
    let connection = Connection { stream };
    Client::handshake(connection).await.map_err(|(err, _)| err)
}
