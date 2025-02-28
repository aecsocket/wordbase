#![doc = include_str!("../README.md")]

use derive_more::{Deref, DerefMut, Display, Error};
pub use wordbase;

use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
use wordbase::{lookup::LookupConfig, protocol};

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

#[derive(Debug, Clone, Copy)]
pub struct Connection<S>(pub S);

impl<S> Connection<S>
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
{
    /// Sends a request and receives a response.
    ///
    /// # Errors
    ///
    /// See [`Error`].
    pub async fn request(
        &mut self,
        request: &protocol::Request,
    ) -> Result<protocol::Response, Error> {
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
        let response = serde_json::from_str::<protocol::Response>(&response)
            .map_err(Error::DeserializeResponse)?;
        Ok(response)
    }

    /// Sends a [`protocol::Request::FetchLookupConfig`].
    ///
    /// # Errors
    ///
    /// See [`Error`].
    pub async fn fetch_lookup_config(&mut self) -> Result<LookupConfig, Error> {
        if let protocol::Response::LookupConfig(res) =
            self.request(&protocol::Request::FetchLookupConfig).await?
        {
            Ok(res)
        } else {
            Err(Error::InvalidResponseKind)
        }
    }

    /// Sends a [`protocol::Request::Lookup`].
    ///
    /// # Errors
    ///
    /// See [`Error`].
    pub async fn lookup(
        &mut self,
        request: protocol::LookupRequest,
    ) -> Result<protocol::LookupResponse, Error> {
        if let protocol::Response::Lookup(res) = self.request(&request.into()).await? {
            Ok(res)
        } else {
            Err(Error::InvalidResponseKind)
        }
    }
}

/// Wordbase client connection over [`tokio_tungstenite`].
///
/// Use [`connect`] to connect to a server, or use  [`Client::handshake`] to
/// create one from an existing [`tokio_tungstenite::WebSocketStream`].
#[derive(Debug, Clone, Deref, DerefMut)]
pub struct Client<S> {
    #[deref]
    #[deref_mut]
    connection: Connection<S>,
    lookup_config: LookupConfig,
}

impl<S> Client<S>
where
    S: Stream<Item = Result<Message, WsError>> + Sink<Message, Error = WsError> + Unpin,
{
    /// Creates a new [`Connection`] by performing a handshake with an existing
    /// stream.
    ///
    /// This handshake will also fetch the server's configuration, which will be
    /// accessible via this client.
    ///
    /// # Errors
    ///
    /// If handshaking fails, this returns an error as well as the original stream.
    pub async fn handshake(stream: S) -> Result<Self, (Error, S)> {
        let mut connection = Connection(stream);
        let lookup_config = match connection.fetch_lookup_config().await {
            Ok(config) => Ok(config),
            Err(err) => {
                return Err((err, connection.0));
            }
        }?;

        Ok(Self {
            connection,
            lookup_config,
        })
    }

    /// Gets a shared reference to the underlying [`Connection`].
    pub const fn connection(&self) -> &Connection<S> {
        &self.connection
    }

    /// Gets a mutable reference to the underlying [`Connection`].
    pub fn connection_mut(&mut self) -> &mut Connection<S> {
        &mut self.connection
    }

    /// Takes the underlying [`Connection`].
    pub fn into_connection(self) -> Connection<S> {
        self.connection
    }

    /// Gets the server's [`LookupConfig`].
    pub const fn lookup_config(&self) -> &LookupConfig {
        &self.lookup_config
    }
}

/// Connects to a Wordbase server at the given URL and sets up a [`Client`].
///
/// See [`tokio_tungstenite::connect_async`] and [`Client::handshake`].
///
/// # Errors
///
/// See [`WsError`].
pub async fn connect(
    request: impl IntoClientRequest + Unpin,
) -> Result<Client<WebSocketStream<MaybeTlsStream<TcpStream>>>, Error> {
    let (stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(Error::Connect)?;
    Client::handshake(stream).await.map_err(|(err, _)| err)
}
