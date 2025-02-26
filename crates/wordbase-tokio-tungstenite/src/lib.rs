#![doc = include_str!("../README.md")]

use derive_more::{Display, Error};
pub use wordbase;

use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
use wordbase::{request, response};

/// Wordbase client connection over [`tokio_tungstenite`].
///
/// Wrap an existing [`tokio_tungstenite::WebSocketStream`] in this type, or use
/// [`connect`] to create a new connection to a Wordbase server.
#[derive(Debug, Clone, Copy)]
pub struct Connection<S>(S);

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
    SerializeRequest(serde_json::Error),
    /// Failed to send request to the server.
    #[display("failed to send request")]
    Send(WsError),
    /// WebSocket stream is closed.
    #[display("stream closed")]
    StreamClosed,
    /// WebSocket stream error.
    #[display("stream error")]
    StreamError(WsError),
    /// Failed to parse server response as UTF-8 text.
    #[display("failed to parse response as UTF-8 text")]
    ResponseIntoText(WsError),
    /// Failed to deserialize server response.
    #[display("failed to deserialize response")]
    DeserializeResponse(serde_json::Error),
    /// Received invalid response kind from server.
    #[display("received invalid response kind")]
    InvalidResponseKind,
}

/// Connects to a Wordbase server at the given URL.
///
/// See [`tokio_tungstenite::connect_async`].
///
/// # Errors
///
/// See [`WsError`].
pub async fn connect<R: IntoClientRequest + Unpin>(
    request: R,
) -> Result<Connection<WebSocketStream<MaybeTlsStream<TcpStream>>>, Error> {
    let (stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(Error::Connect)?;
    Connection::handshake(stream).await.map_err(|(err, _)| err)
}

impl<S> Connection<S>
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
{
    /// Creates a new [`Connection`] by performing a handshake with an existing
    /// stream.
    ///
    /// # Errors
    ///
    /// If handshaking fails, this returns an error as well as the original stream.
    pub async fn handshake(stream: S) -> Result<Self, (Error, S)> {
        let mut connection = Self(stream);
        match connection.request(&wordbase::Request::Ping).await {
            Ok(wordbase::Response::Pong(_)) => Ok(connection),
            Ok(_) => Err((Error::InvalidResponseKind, connection.0)),
            Err(err) => Err((err, connection.0)),
        }
    }

    /// Creates a new [`Connection`] form an existing stream, assuming that a
    /// handshake has already been performed.
    pub const fn assume_handshaked(stream: S) -> Self {
        Self(stream)
    }

    /// Gets a shared reference to the underlying stream.
    pub const fn inner(&self) -> &S {
        &self.0
    }

    /// Gets a mutable reference to the underlying stream.
    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.0
    }

    /// Takes the underlying stream.
    pub fn into_inner(self) -> S {
        self.0
    }

    /// Sends a request and receives a response.
    ///
    /// # Errors
    ///
    /// See [`Error`].
    pub async fn request(
        &mut self,
        request: &wordbase::Request,
    ) -> Result<response::Response, Error> {
        let request = serde_json::to_string(request).map_err(Error::SerializeRequest)?;
        self.0
            .send(Message::text(request))
            .await
            .map_err(Error::Send)?;
        let response = self
            .0
            .next()
            .await
            .ok_or(Error::StreamClosed)?
            .map_err(Error::StreamError)?
            .into_text()
            .map_err(Error::ResponseIntoText)?;
        let response = serde_json::from_str::<wordbase::Response>(&response)
            .map_err(Error::DeserializeResponse)?;
        Ok(response)
    }

    /// Sends a [`request::Ping`].
    ///
    /// # Errors
    ///
    /// See [`Error`].
    pub async fn ping(&mut self) -> Result<response::Pong, Error> {
        if let wordbase::Response::Pong(pong) = self.request(&wordbase::Request::Ping).await? {
            Ok(pong)
        } else {
            Err(Error::InvalidResponseKind)
        }
    }

    /// Sends a [`request::Lookup`].
    ///
    /// # Errors
    ///
    /// See [`Error`].
    pub async fn lookup(&mut self, text: impl Into<String>) -> Result<response::Lookup, Error> {
        let request = request::Lookup { text: text.into() };
        if let wordbase::Response::Lookup(lookup) = self.request(&request.into()).await? {
            Ok(lookup)
        } else {
            Err(Error::InvalidResponseKind)
        }
    }
}
