use core::task;
use std::{num::Wrapping, pin::Pin, sync::Arc};

use anyhow::{Context, Result};
use futures::{Sink, SinkExt, Stream, StreamExt, never::Never};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc, oneshot},
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{
        Message, Utf8Bytes,
        protocol::{CloseFrame, frame::coding::CloseCode},
    },
};
use tracing::{Instrument, info, info_span, trace};
use wordbase::protocol::{self, FromClient, FromServer, Lookup, LookupInfo, NewSentence};

use crate::{
    Config,
    mecab::{MecabRequest, MecabResponse},
};

pub async fn run(
    config: Arc<Config>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
    recv_new_sentence: broadcast::Receiver<NewSentence>,
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
        let recv_new_sentence = recv_new_sentence.resubscribe();
        tokio::spawn(
            async move {
                info!("Incoming connection from {peer_addr:?}");
                let Err(err) =
                    handle_stream(config, send_mecab_request, recv_new_sentence, stream).await;
                info!("Connection lost: {err:?}");
            }
            .instrument(info_span!("connection", id = %connection_id)),
        );
        connection_id += 1;
    }
}

struct Connection(WebSocketStream<TcpStream>);

impl Connection {
    async fn read(&mut self) -> Result<FromClient> {
        let message = self
            .0
            .next()
            .await
            .context("stream closed")?
            .context("stream error")?
            .into_text()
            .context("received message which was not UTF-8 text")?;
        let message =
            serde_json::from_str::<FromClient>(&message).context("received invalid message")?;
        Ok(message)
    }

    async fn write(&mut self, message: &FromServer) -> Result<()> {
        let message = serde_json::to_string(message).context("failed to serialize message")?;
        self.0.send(Message::text(message)).await?;
        Ok(())
    }
}

async fn handle_stream(
    config: Arc<Config>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
    mut recv_new_sentence: broadcast::Receiver<NewSentence>,
    stream: TcpStream,
) -> Result<Never> {
    let stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to accept WebSocket stream")?;
    let mut connection = Connection(stream);

    loop {
        tokio::select! {
            Ok(new_sentence) = recv_new_sentence.recv() => {
                on_new_sentence(&mut connection, new_sentence).await;
            }
            message = connection.read() => {

            }
        }

        // let message = stream
        //     .next()
        //     .await
        //     .context("stream closed")?
        //     .context("stream error")?;

        // if let Err(err) = handle_message(&config, &send_mecab_request, &mut stream, message).await {
        //     let close_frame = CloseFrame {
        //         code: CloseCode::Abnormal,
        //         reason: Utf8Bytes::from(err.to_string()),
        //     };
        //     tokio::spawn(async move {
        //         _ = stream.close(Some(close_frame)).await;
        //     });
        //     return Err(err);
        // }
    }
}

async fn on_new_sentence(connection: &mut Connection, new_sentence: NewSentence) {
    _ = connection
        .write(&FromServer::NewSentence(new_sentence))
        .await;
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

    let request = serde_json::from_str::<protocol::FromClient>(&message)
        .context("received invalid request")?;

    trace!("Requested {request:?}");
    let response: protocol::ResponseKind = match request {
        protocol::FromClient::FetchLookupConfig => {
            protocol::ResponseKind::FetchLookupConfig(config.lookup.clone())
        }
        protocol::FromClient::Lookup(request) => protocol::ResponseKind::Lookup {
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
    request: Lookup,
    send_mecab_request: &mpsc::Sender<MecabRequest>,
) -> Result<Option<LookupInfo>> {
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
