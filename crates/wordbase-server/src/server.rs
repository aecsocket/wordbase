use std::{num::Wrapping, str::FromStr, sync::Arc, time::Duration};

use anyhow::{Context as _, Result, bail};
use derive_more::{Deref, DerefMut};
use futures::{SinkExt as _, StreamExt as _, never::Never};
use sqlx::{
    Pool, Sqlite,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc, oneshot},
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use tracing::{Instrument, info, info_span, warn};
use wordbase::{
    protocol::{FromClient, FromServer, NewSentence},
    schema::LookupInfo,
};

use crate::{
    Config, db, import,
    mecab::{MecabInfo, MecabRequest},
};

pub async fn run(
    config: Arc<Config>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
    send_new_sentence: broadcast::Sender<NewSentence>,
) -> Result<Never> {
    let db = SqlitePoolOptions::new()
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(300))
        .connect_with(
            SqliteConnectOptions::from_str("sqlite://wordbase.db")?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await
        .context("failed to connect to database")?;
    info!("Connected to SQLite database");

    {
        // TODO
        sqlx::query(include_str!("setup_db.sql"))
            .execute(&db)
            .await
            .context("failed to set up database")?;

        // let jitendex = tokio::spawn(
        //     import::from_yomitan(db.clone(), "/home/dev/dictionaries/jitendex.zip")
        //         .instrument(info_span!("import", path = "jitendex.zip")),
        // );
        // let jpdb = tokio::spawn(
        //     import::from_yomitan(db.clone(), "/home/dev/dictionaries/jpdb.zip")
        //         .instrument(info_span!("import", path = "jpdb.zip")),
        // );
        // let nhk = tokio::spawn(
        //     import::from_yomitan(db.clone(), "/home/dev/dictionaries/nhk.zip")
        //         .instrument(info_span!("import", path = "nhk.zip")),
        // );
        let jmnedict = tokio::spawn(
            import::from_yomitan(db.clone(), "/home/dev/dictionaries/jmnedict.zip")
                .instrument(info_span!("import", path = "jmnedict.zip")),
        );

        let (jmnedict,) = tokio::try_join!(jmnedict).context("failed to import")?;
        // jitendex.context("failed to import jitendex")?;
        // jpdb.context("failed to import jpdb")?;
        // nhk.context("failed to import nhk")?;
        jmnedict.context("failed to import jmnedict")?;
    }

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
        let db = db.clone();
        let send_mecab_request = send_mecab_request.clone();
        let send_new_sentence = send_new_sentence.clone();
        tokio::spawn(
            async move {
                info!("Incoming connection from {peer_addr:?}");
                let Err(err) =
                    handle_stream(config, db, send_mecab_request, send_new_sentence, stream).await;
                info!("Connection lost: {err:?}");
            }
            .instrument(info_span!("connection", id = %connection_id)),
        );
        connection_id += 1;
    }
}

#[derive(Debug, Deref, DerefMut)]
struct Connection(WebSocketStream<TcpStream>);

impl Connection {
    async fn write(&mut self, message: &FromServer) -> Result<()> {
        let message = serde_json::to_string(message).context("failed to serialize message")?;
        self.send(Message::text(message)).await?;
        Ok(())
    }
}

async fn handle_stream(
    config: Arc<Config>,
    db: Pool<Sqlite>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
    send_new_sentence: broadcast::Sender<NewSentence>,
    stream: TcpStream,
) -> Result<Never> {
    let stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to accept WebSocket stream")?;
    let mut connection = Connection(stream);
    let mut recv_new_sentence = send_new_sentence.subscribe();

    connection
        .write(&FromServer::SyncConfig {
            config: config.shared.clone(),
        })
        .await
        .context("failed to sync config")?;

    loop {
        tokio::select! {
            Ok(new_sentence) = recv_new_sentence.recv() => {
                forward_new_sentence(&mut connection, new_sentence).await;
            }
            data = connection.next() => {
                let data = data
                    .context("stream closed")?
                    .context("stream error")?;
                if let Err(err) = handle_message(
                    &config,
                    &db,
                    &send_mecab_request,
                    &send_new_sentence,
                    &mut connection,
                    data,
                )
                .await
                {
                    warn!("Failed to handle request: {err:?}");
                    let message = format!("{err:?}");
                    _ = connection.write(&FromServer::Error { message }).await;
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
    db: &Pool<Sqlite>,
    send_mecab_request: &mpsc::Sender<MecabRequest>,
    send_new_sentence: &broadcast::Sender<NewSentence>,
    connection: &mut Connection,
    data: Message,
) -> Result<()> {
    let data = data.into_data();
    if data.is_empty() {
        return Ok(());
    }
    let message =
        serde_json::from_slice::<FromClient>(&data).context("received invalid message")?;

    match message {
        FromClient::NewSentence(new_sentence) => {
            _ = send_new_sentence.send(new_sentence);
            Ok(())
        }
        FromClient::Lookup { text } => {
            let lookup = do_lookup(config, db, send_mecab_request, text)
                .await
                .context("failed to perform lookup")?;
            connection
                .write(&FromServer::Lookup { lookup })
                .await
                .context("failed to send response")
        }
        FromClient::ListDictionaries => {
            let dictionaries = db::list_dictionaries(db)
                .await
                .context("failed to list dictionaries")?;
            connection
                .write(&FromServer::ListDictionaries { dictionaries })
                .await
                .context("failed to send response")
        }
        FromClient::RemoveDictionary { dictionary_id } => {
            let result = db::remove_dictionary(db, dictionary_id)
                .await
                .context("failed to remove dictionary")?;
            connection
                .write(&FromServer::RemoveDictionary { result })
                .await
                .context("failed to send response")
        }
        FromClient::SetDictionaryEnabled {
            dictionary_id,
            enabled,
        } => {
            let result = db::set_dictionary_enabled(db, dictionary_id, enabled)
                .await
                .context("failed to set dictionary enabled state")?;
            connection
                .write(&FromServer::SetDictionaryEnabled { result })
                .await
                .context("failed to send response")
        }
    }
}

async fn do_lookup(
    config: &Config,
    db: &Pool<Sqlite>,
    send_mecab_request: &mpsc::Sender<MecabRequest>,
    text: String,
) -> Result<Option<LookupInfo>> {
    let request_len = text.chars().count();
    let max_request_len = config.shared.max_lookup_len;
    let request_len_valid =
        u16::try_from(request_len).is_ok_and(|request_len| request_len <= max_request_len);
    if !request_len_valid {
        bail!("request too long - {request_len} / {max_request_len} characters");
    }

    let (send_mecab_response, recv_mecab_response) = oneshot::channel::<Option<MecabInfo>>();
    _ = send_mecab_request
        .send(MecabRequest {
            text,
            send_info: send_mecab_response,
        })
        .await;
    let Some(mecab) = recv_mecab_response.await.context("mecab channel dropped")? else {
        return Ok(None);
    };

    let info = db::lookup(db, mecab.lemma).await?;
    Ok(Some(info))
}
