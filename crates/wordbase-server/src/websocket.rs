use std::{num::Wrapping, sync::Arc};

use anyhow::{Context as _, Result, bail};
use derive_more::{Deref, DerefMut};
use futures::{SinkExt as _, StreamExt as _, never::Never};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc, oneshot},
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use tracing::{Instrument, info, info_span};
use wordbase::{
    lookup::LookupInfo,
    protocol::{ClientRequest, FromClient, FromServer, Lookup, NewSentence, Response},
};

use crate::{
    Config,
    mecab::{MecabRequest, MecabResponse},
};

pub async fn run(
    config: Arc<Config>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
    send_new_sentence: broadcast::Sender<NewSentence>,
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
        let send_new_sentence = send_new_sentence.clone();
        tokio::spawn(
            async move {
                info!("Incoming connection from {peer_addr:?}");
                let Err(err) =
                    handle_stream(config, send_mecab_request, send_new_sentence, stream).await;
                info!("Connection lost: {err:?}");
            }
            .instrument(info_span!("connection", id = %connection_id)),
        );
        connection_id += 1;
    }
}

#[derive(Debug, Deref, DerefMut)]
struct Connection {
    stream: WebSocketStream<TcpStream>,
}

impl Connection {
    async fn write(&mut self, message: &FromServer) -> Result<()> {
        let message = serde_json::to_string(message).context("failed to serialize message")?;
        self.send(Message::text(message)).await?;
        Ok(())
    }
}

async fn handle_stream(
    config: Arc<Config>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
    send_new_sentence: broadcast::Sender<NewSentence>,
    stream: TcpStream,
) -> Result<Never> {
    let stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to accept WebSocket stream")?;
    let mut connection = Connection { stream };
    let mut recv_new_sentence = send_new_sentence.subscribe();

    loop {
        tokio::select! {
            Ok(new_sentence) = recv_new_sentence.recv() => {
                forward_new_sentence(&mut connection, new_sentence).await;
            }
            message = connection.stream.next() => {
                let message = message
                    .context("stream closed")?
                    .context("stream error")?;
                if let Err(err) = handle_message(&config, &send_mecab_request, &send_new_sentence, &mut connection, message).await {
                    connection.write(&FromServer::Error(format!("{err:?}"))).await?;
                }
            }
        }
    }
}

async fn forward_new_sentence(connection: &mut Connection, new_sentence: NewSentence) {
    _ = connection
        .write(&FromServer::NewSentence(new_sentence))
        .await;
}

async fn handle_message(
    config: &Config,
    send_mecab_request: &mpsc::Sender<MecabRequest>,
    send_new_sentence: &broadcast::Sender<NewSentence>,
    connection: &mut Connection,
    message: Message,
) -> Result<()> {
    let message = message
        .into_text()
        .context("received message which was not UTF-8 text")?;
    let message =
        serde_json::from_str::<FromClient>(&message).context("received invalid message")?;

    let request_id = message.request_id;
    match message.request {
        ClientRequest::Lookup(request) => {
            let response = do_lookup(config, send_mecab_request, request).await?;
            connection
                .write(&FromServer::Response {
                    request_id,
                    response: Response::LookupInfo(response),
                })
                .await?;
        }
        ClientRequest::AddAnkiNote(request) => {}
        ClientRequest::NewSentence(new_sentence) => {
            _ = send_new_sentence.send(new_sentence);
        }
    }

    Ok(())
}

async fn do_lookup(
    config: &Config,
    send_mecab_request: &mpsc::Sender<MecabRequest>,
    request: Lookup,
) -> Result<Option<LookupInfo>> {
    let request_len_valid = u64::try_from(request.text.chars().count())
        .is_ok_and(|request_len| request_len <= config.lookup.max_request_len);
    if !request_len_valid {
        bail!("request too long");
    }

    let (send_mecab_response, recv_mecab_response) = oneshot::channel::<Option<MecabResponse>>();
    _ = send_mecab_request
        .send(MecabRequest {
            text: request.text,
            send_response: send_mecab_response,
        })
        .await;
    let Some(mecab_response) = recv_mecab_response.await.context("mecab channel dropped")? else {
        return Ok(None);
    };

    Ok(Some(LookupInfo {
        conjugated_len: mecab_response.conjugated_len,
        base_form: mecab_response.base_form,
    }))
}
