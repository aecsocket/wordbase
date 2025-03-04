#![doc = include_str!("../README.md")]

use std::num::Wrapping;

use derive_more::{Deref, DerefMut, Display, Error};
pub use wordbase;

use futures::{Sink, SinkExt, Stream, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream, WebSocketStream,
    tungstenite::{Message, client::IntoClientRequest},
};
use wordbase::{
    lookup::{LookupConfig, LookupInfo},
    protocol::{ClientRequest, FromClient, FromServer, Lookup, RequestId, Response},
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
    FromServer(#[error(ignore)] String),
}

#[derive(Debug)]
pub struct Connection {
    pub stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    lookup_config: LookupConfig,
    next_request_id: Wrapping<u64>,
}

impl Connection {
    pub async fn lookup(&mut self, request: Lookup) -> Result<Option<LookupInfo>, Error> {
        let this_request_id = RequestId::from_raw(self.next_request_id.0);
        let request = serde_json::to_string(&FromClient {
            request_id: this_request_id,
            request: ClientRequest::from(request),
        })
        .map_err(Error::SerializeRequest)?;
        self.next_request_id += 1;

        self.stream
            .send(Message::text(request))
            .await
            .map_err(Error::Send)?;

        loop {
            let message = self
                .stream
                .next()
                .await
                .ok_or(Error::StreamClosed)?
                .map_err(Error::StreamError)?
                .into_text()
                .map_err(Error::ResponseIntoText)?;
            let message =
                serde_json::from_str::<FromServer>(&message).map_err(Error::DeserializeResponse)?;

            match message {
                FromServer::SyncLookupConfig(config) => {
                    todo!();
                }
                FromServer::NewSentence(new_sentence) => {
                    todo!();
                }
                FromServer::Response {
                    request_id,
                    response,
                } if request_id == this_request_id => {
                    return if let Response::LookupInfo(response) = response {
                        Ok(response)
                    } else {
                        Err(Error::InvalidResponseKind)
                    };
                }
                FromServer::Response { .. } => {
                    todo!();
                }
                FromServer::Error(err) => {
                    return Err(Error::FromServer(err));
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
pub async fn connect(request: impl IntoClientRequest + Unpin) -> Result<Connection, Error> {
    let (stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .map_err(Error::Connect)?;
    Ok(Connection {
        stream,
        lookup_config: LookupConfig::default(),
        next_request_id: Wrapping(0),
    })
}
