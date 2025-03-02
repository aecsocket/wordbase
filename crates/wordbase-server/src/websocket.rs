use std::{num::Wrapping, sync::Arc};

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt, never::Never};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{mpsc, oneshot},
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Message, Utf8Bytes,
        protocol::{CloseFrame, frame::coding::CloseCode},
    },
};
use tracing::{Instrument, info, info_span, trace};
use wordbase::protocol::{self, Lookup, LookupResponse};

use crate::{
    Config,
    mecab::{MecabRequest, MecabResponse},
};

pub async fn run(
    config: Arc<Config>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
) -> Result<Never> {
    let listener = TcpListener::bind(&config.listen_addr)
        .await
        .context("failed to bind TCP listener")?;
    info!("Listening on {:?}", config.listen_addr);

    let mut connection_id = Wrapping(0usize);
    loop {
        let (stream, peer_addr) = listener
            .accept()
            .await
            .context("failed to accept TCP stream")?;

        let config = config.clone();
        let send_mecab_request = send_mecab_request.clone();
        tokio::spawn(
            async move {
                info!("Incoming connection from {peer_addr:?}");
                let Err(err) = handle_stream(config, send_mecab_request, stream).await;
                info!("Connection lost: {err:?}");
            }
            .instrument(info_span!("connection", id = %connection_id)),
        );

        connection_id += 1;
    }
}

async fn handle_stream(
    config: Arc<Config>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
    stream: TcpStream,
) -> Result<Never> {
    let mut stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to accept WebSocket stream")?;

    loop {
        let message = stream
            .next()
            .await
            .context("stream closed")?
            .context("stream error")?;

        if let Err(err) = handle_message(&config, &send_mecab_request, &mut stream, message).await {
            let close_frame = CloseFrame {
                code: CloseCode::Abnormal,
                reason: Utf8Bytes::from(err.to_string()),
            };
            tokio::spawn(async move {
                _ = stream.close(Some(close_frame)).await;
            });
            return Err(err);
        }
    }
}

async fn handle_message(
    config: &Config,
    send_mecab_request: &mpsc::Sender<MecabRequest>,
    stream: &mut WebSocketStream<TcpStream>,
    message: Message,
) -> Result<()> {
    let message = message
        .into_text()
        .context("received message which is not UTF-8 text")?;
    if message.is_empty() {
        return Ok(());
    }

    let request =
        serde_json::from_str::<protocol::Request>(&message).context("received invalid request")?;

    trace!("Requested {request:?}");
    let response: protocol::Response = match request {
        protocol::Request::FetchLookupConfig => {
            protocol::Response::LookupConfig(config.lookup.clone())
        }
        protocol::Request::Lookup(request) => protocol::Response::Lookup {
            response: lookup(request, send_mecab_request).await?,
        },
    };

    let response = serde_json::to_string(&response).context("failed to serialize response")?;
    stream
        .send(Message::from(response))
        .await
        .context("failed to send response")?;

    Ok(())
}

async fn lookup(
    request: protocol::LookupRequest,
    send_mecab_request: &mpsc::Sender<MecabRequest>,
) -> Result<Option<LookupResponse>> {
    let (send_mecab_response, recv_mecab_response) = oneshot::channel::<MecabResponse>();
    _ = send_mecab_request
        .send(MecabRequest {
            text: request.text,
            send_response: send_mecab_response,
        })
        .await;
    let mecab_response = recv_mecab_response.await.context("mecab channel dropped")?;

    if let Some(deinflected) = mecab_response.deinflected {
        Ok(Some(LookupResponse {
            raw: Lookup {
                chars_scanned: 3,
                entries: deinflected,
            },
            html: None,
        }))
    } else {
        Ok(None)
    }
}
