#![doc = include_str!("../README.md")]

use anyhow::{Context, Result};
use futures::{SinkExt, StreamExt, never::Never};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::broadcast,
};
use tokio_tungstenite::{
    WebSocketStream,
    tungstenite::{Message, Utf8Bytes},
};
use wordbase::TexthookerSentence;

const PROCESS_PATH: &str = "wordbase-texthooker-server";

#[tokio::main]
async fn main() -> Result<()> {
    let (send_sentence, recv_sentence) = broadcast::channel(4);
    tokio::spawn(async move {
        server(recv_sentence).await;
    });

    let mut rl = rustyline::DefaultEditor::new().context("failed to create readline editor")?;
    loop {
        let sentence = rl.readline("> ").unwrap();
        send_sentence
            .send(sentence)
            .expect("sentence channel closed");
    }
}

async fn server(recv_sentence: broadcast::Receiver<String>) -> Result<Never> {
    let listener = TcpListener::bind("127.0.0.1:9001")
        .await
        .context("failed to bind socket")?;
    loop {
        let (stream, peer_addr) = listener
            .accept()
            .await
            .context("failed to accept connection")?;
        eprintln!("Connection from {peer_addr}");

        let recv_sentence = recv_sentence.resubscribe();
        tokio::spawn(async move {
            let Err(err) = handle_stream(stream, recv_sentence).await;
            eprintln!("Connection from {peer_addr} closed: {err:?}");
        });
    }
}

async fn handle_stream(
    stream: TcpStream,
    mut recv_sentence: broadcast::Receiver<String>,
) -> Result<Never> {
    let (mut stream_send, _) = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to create WebSocket stream")?
        .split();

    loop {
        let sentence = recv_sentence
            .recv()
            .await
            .context("sentence channel closed")?;
        let sentence = serde_json::to_string(&TexthookerSentence {
            process_path: PROCESS_PATH.into(),
            sentence,
        })
        .expect("should be able to serialize sentence");
        stream_send
            .send(Message::Text(Utf8Bytes::from(sentence)))
            .await
            .context("failed to send sentence")?;
    }
}
