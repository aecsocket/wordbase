use {
    crate::{
        Config, Event, dictionary,
        import::{self, ImportEvent},
        mecab::{MecabInfo, MecabRequest},
        term,
    },
    anyhow::{Context as _, Result, bail},
    derive_more::{Deref, DerefMut},
    futures::{SinkExt as _, StreamExt as _, never::Never},
    sqlx::{
        Pool, Sqlite,
        sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    },
    std::{num::Wrapping, str::FromStr, sync::Arc, time::Duration},
    tokio::{
        net::{TcpListener, TcpStream},
        sync::{broadcast, mpsc, oneshot},
        task::JoinSet,
    },
    tokio_tungstenite::{WebSocketStream, tungstenite::Message},
    tracing::{Instrument, info, info_span, warn},
    wordbase::protocol::{FromClient, FromServer, LookupRequest},
};

pub async fn run(
    config: Arc<Config>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
    send_event: broadcast::Sender<Event>,
) -> Result<Never> {
    let db = SqlitePoolOptions::new()
    // todo fix
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(99999))
        .connect_with(
            SqliteConnectOptions::from_str("sqlite://wordbase.db")?
                .create_if_missing(true)
                .journal_mode(SqliteJournalMode::Wal),
        )
        .await
        .context("failed to connect to database")?;
    sqlx::query(include_str!("setup_db.sql"))
        .execute(&db)
        .await
        .context("failed to set up database")?;
    info!("Connected to database");

    // TODO
    const IMPORTS: &[&str] = &[
        // "1. jitendex-yomitan.zip",
        "2. JMnedict.zip",
        "11. [Pitch] NHK 2016.zip",
        "12. JPDB_v2.2_Frequency_Kana_2024-10-13.zip",
    ];
    // const IMPORTS: &[&str] = &[
    //     "1. jitendex-yomitan.zip",
    //     "2. JMnedict.zip",
    //     "3. [Grammar] Dictionary of Japanese Grammar 日本語文法辞典
    // (Recommended).zip",     "4. [Monolingual] 三省堂国語辞典　第八版
    // (Recommended).zip",     "5. [JA-JA] 明鏡国語辞典　第二版_2023_07_22.zip",
    //     "6. 漢字ペディア同訓異義.zip",
    //     "7. [Monolingual] デジタル大辞泉.zip",
    //     "8. [Monolingual] PixivLight.zip",
    //     "9. [Monolingual] 実用日本語表現辞典 Extended (Recommended).zip",
    //     "10. kanjiten.zip",
    //     "11. [Pitch] NHK 2016.zip",
    //     "12. JPDB_v2.2_Frequency_Kana_2024-10-13.zip",
    //     "13. [Freq] VN Freq v2.zip",
    //     "14. [Freq] Novels.zip",
    //     "15. [Freq] Anime & J-drama.zip",
    //     "16. [JA Freq] YoutubeFreqV3.zip",
    //     "17. [JA Freq] Wikipedia v2.zip",
    //     "18. BCCWJ_SUW_LUW_combined.zip",
    //     "19. [Freq] CC100.zip",
    //     "20. [Freq] InnocentRanked.zip",
    //     "21. [Freq] Narou Freq.zip",
    // ];

    let mut joins = JoinSet::new();
    for path in IMPORTS {
        let span = info_span!("import", %path);
        let (send_event, recv_event) = mpsc::channel::<ImportEvent>(4);
        joins.spawn(
            async move {
                import::yomitan(
                    db.clone(),
                    format!("/home/dev/all-dictionaries/{path}"),
                    send_event,
                )
                .await;
            }
            .instrument(span.clone()),
        );
        joins.spawn(
            async move {
                while let Some(event) = recv_event.recv().await {
                    match event {
                        ImportEvent::ReadToMemory => {
                            info!("Read to memory");
                        }
                        ImportEvent::ReadMeta { meta, items_len } => {
                            info!("{} version {} - {items_len} items", meta.)
                        }
                    }
                    info!("foo");
                }
            }
            .instrument(span),
        );
    }
    while let Some(result) = joins.join_next().await {
        result
            .context("cancelled import task")?
            .context("failed to import")?;
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
        let send_event = send_event.clone();
        tokio::spawn(
            async move {
                info!("Incoming connection from {peer_addr:?}");
                let Err(err) =
                    handle_stream(config, db, send_mecab_request, send_event, stream).await;
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
    send_event: broadcast::Sender<Event>,
    stream: TcpStream,
) -> Result<Never> {
    let stream = tokio_tungstenite::accept_async(stream)
        .await
        .context("failed to accept WebSocket stream")?;
    let mut connection = Connection(stream);
    let mut recv_event = send_event.subscribe();

    let dictionaries = dictionary::all(&db)
        .await
        .context("failed to fetch dictionaries")?;

    connection
        .write(&FromServer::SyncLookupConfig {
            lookup_config: config.lookup.clone(),
        })
        .await
        .context("failed to sync lookup config")?;
    connection
        .write(&FromServer::SyncDictionaries { dictionaries })
        .await
        .context("failed to sync dictionaries")?;

    loop {
        tokio::select! {
            Ok(event) = recv_event.recv() => {
                forward_event(&mut connection, event).await;
            }
            data = connection.next() => {
                let data = data
                    .context("stream closed")?
                    .context("stream error")?;
                if let Err(err) = handle_message(
                    &config,
                    &db,
                    &send_mecab_request,
                    &send_event,
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

async fn forward_event(connection: &mut Connection, event: Event) {
    let message = match event {
        Event::HookSentence(sentencece) => FromServer::HookSentence(sentencece),
        Event::SyncDictionaries(dictionaries) => FromServer::SyncDictionaries { dictionaries },
    };

    _ = connection.write(&message).await;
}

async fn handle_message(
    config: &Config,
    db: &Pool<Sqlite>,
    send_mecab_request: &mpsc::Sender<MecabRequest>,
    send_event: &broadcast::Sender<Event>,
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
        FromClient::HookSentence(sentence) => {
            _ = send_event.send(Event::HookSentence(sentence));
            Ok(())
        }
        FromClient::Lookup(request) => {
            do_lookup(config, db, send_mecab_request, connection, request)
                .await
                .context("failed to perform lookup")?;
            connection
                .write(&FromServer::LookupDone)
                .await
                .context("failed to send final response")?;
            Ok(())
        }
        FromClient::RemoveDictionary { dictionary_id } => {
            let result = dictionary::remove(db, dictionary_id)
                .await
                .context("failed to remove dictionary")?;
            connection
                .write(&FromServer::RemoveDictionary { result })
                .await
                .context("failed to send response")?;
            send_dictionary_sync(db, send_event)
                .await
                .context("failed to send dictionary sync")?;
            Ok(())
        }
        FromClient::SetDictionaryEnabled {
            dictionary_id,
            enabled,
        } => {
            let result = dictionary::set_enabled(db, dictionary_id, enabled)
                .await
                .context("failed to set dictionary enabled state")?;
            connection
                .write(&FromServer::SetDictionaryEnabled { result })
                .await
                .context("failed to send response")?;
            send_dictionary_sync(db, send_event)
                .await
                .context("failed to send dictionary sync")?;
            Ok(())
        }
    }
}

async fn do_lookup(
    config: &Config,
    db: &Pool<Sqlite>,
    send_mecab_request: &mpsc::Sender<MecabRequest>,
    connection: &mut Connection,
    request: LookupRequest,
) -> Result<()> {
    let LookupRequest {
        text,
        record_kinds: include,
        exclude,
    } = request;

    // count like this instead of using `.count()`
    // because `count` does not short-circuit
    let mut request_chars = text.chars();
    let mut num_request_chars = 0u64;
    let max_request_len = config.lookup.max_request_len;
    while let Some(_) = request_chars.next() {
        num_request_chars += 1;
        if num_request_chars > max_request_len {
            bail!("request too long - max {max_request_len} characters");
        }
    }

    let (send_mecab_info, recv_mecab_info) = oneshot::channel::<Option<MecabInfo>>();
    _ = send_mecab_request
        .send(MecabRequest {
            text,
            send_info: send_mecab_info,
        })
        .await;
    let Some(mecab) = recv_mecab_info.await.context("mecab channel dropped")? else {
        return Ok(());
    };

    let records = term::lookup(db, &mecab.lemma, &include, &exclude)
        .await
        .context("failed to fetch records")?;
    for record in records {
        connection
            .write(&FromServer::Lookup(record))
            .await
            .context("failed to send record")?;
    }
    Ok(())
}

async fn send_dictionary_sync(
    db: &Pool<Sqlite>,
    send_event: &broadcast::Sender<Event>,
) -> Result<()> {
    let dictionaries = dictionary::all(db)
        .await
        .context("failed to fetch dictionaries")?;

    _ = send_event.send(Event::SyncDictionaries(dictionaries));
    Ok(())
}
