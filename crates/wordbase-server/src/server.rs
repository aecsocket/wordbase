use std::{
    convert::Infallible,
    io::Cursor,
    num::Wrapping,
    path::Path,
    sync::{
        Arc,
        atomic::{self, AtomicUsize},
    },
};

use anyhow::{Context as _, Result, bail};
use derive_more::{Deref, DerefMut};
use futures::{SinkExt as _, StreamExt as _, never::Never};
use sqlx::{Pool, Sqlite, Transaction, sqlite::SqlitePoolOptions};
use tokio::{
    fs,
    net::{TcpListener, TcpStream},
    runtime,
    sync::{Mutex, broadcast, mpsc, oneshot},
    task::JoinSet,
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use tracing::{Instrument, info, info_span, warn};
use wordbase::{
    dict::{
        Dictionary, DictionaryId, ExpressionEntry, Frequency, FrequencySet, Glossary, GlossarySet,
        Pitch, PitchSet, Reading,
    },
    protocol::{DictionaryNotFound, FromClient, FromServer, LookupInfo, NewSentence},
    yomitan::{self, TermBank, TermMetaBank},
};

use crate::{
    Config,
    mecab::{MecabInfo, MecabRequest},
};

pub async fn run(
    config: Arc<Config>,
    send_mecab_request: mpsc::Sender<MecabRequest>,
    send_new_sentence: broadcast::Sender<NewSentence>,
) -> Result<Never> {
    let db = SqlitePoolOptions::new()
        .max_connections(8)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect to database")?;
    info!("Connected to SQLite database");

    sqlx::query(include_str!("setup_db.sql"))
        .execute(&db)
        .await
        .context("failed to set up database")?;

    let path = Path::new("/home/dev/dictionaries/jitendex.zip");
    todo_import(&db, &path)
        .instrument(info_span!("import", ?path))
        .await
        .context("failed to import")?;

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
                if let Err(err) = handle_message(&config, &db, &send_mecab_request, &send_new_sentence, &mut connection, data).await {
                    warn!("Failed to handle request: {err:?}");
                    _ = connection.write(&FromServer::Error { message: format!("{err:?}") }).await;
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
            connection.write(&FromServer::Lookup { lookup }).await
        }
        FromClient::ListDictionaries => {
            let dictionaries = list_dictionaries(db)
                .await
                .context("failed to list dictionaries")?;
            connection
                .write(&FromServer::ListDictionaries { dictionaries })
                .await
        }
        FromClient::RemoveDictionary { dictionary_id } => {
            let result = remove_dictionary(db, dictionary_id)
                .await
                .context("failed to remove dictionary")?;
            connection
                .write(&FromServer::RemoveDictionary { result })
                .await
        }
    }
}

async fn list_dictionaries(db: &Pool<Sqlite>) -> Result<Vec<Dictionary>> {
    let dictionaries = sqlx::query!("SELECT id, title, revision FROM dictionaries")
        .fetch(db)
        .map(|record| {
            let record = record.context("failed to fetch database record")?;
            Ok::<_, anyhow::Error>(Dictionary {
                id: DictionaryId(record.id),
                title: record.title,
                revision: record.revision,
            })
        })
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    Ok(dictionaries)
}

async fn remove_dictionary(
    db: &Pool<Sqlite>,
    dictionary_id: DictionaryId,
) -> Result<Result<(), DictionaryNotFound>> {
    let result = sqlx::query!("DELETE FROM dictionaries WHERE id = $1", dictionary_id.0)
        .execute(db)
        .await
        .context("failed to delete database record")?;
    Ok(if result.rows_affected() > 0 {
        Ok(())
    } else {
        Err(DictionaryNotFound)
    })
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

    let expressions = sqlx::query!(
        "SELECT expression, reading FROM terms WHERE expression = $1 OR reading = $1",
        mecab.lemma
    )
    .fetch(db)
    .map(|record| {
        let record = record.context("failed to fetch database record")?;
        Ok::<_, anyhow::Error>(ExpressionEntry {
            reading: Reading::from_no_pairs(record.expression, record.reading),
            frequency_sets: vec![],
            pitch_sets: vec![],
            glossary_sets: vec![],
        })
    })
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .collect::<Result<Vec<_>, _>>()
    .context("failed to fetch expressions")?;

    Ok(Some(LookupInfo {
        lemma: mecab.lemma,
        expressions,
    }))
}

async fn todo_import(db: &Pool<Sqlite>, path: impl AsRef<Path>) -> Result<()> {
    info!("Reading archive into memory");
    let archive = fs::read(path)
        .await
        .context("failed to read file into memory")?;

    let (parser, index) = yomitan::Parse::new(|| Ok::<_, Infallible>(Cursor::new(&archive)))
        .context("failed to parse")?;
    let term_banks_left = AtomicUsize::new(parser.term_banks().len());
    let term_meta_banks_left = AtomicUsize::new(parser.term_meta_banks().len());

    info!("{} rev {}:", index.title, index.revision);
    info!("  - {} tag banks", parser.tag_banks().len());
    info!(
        "  - {} term banks",
        term_banks_left.load(atomic::Ordering::SeqCst)
    );
    info!(
        "  - {} term meta banks",
        term_meta_banks_left.load(atomic::Ordering::SeqCst)
    );
    info!("  - {} kanji banks", parser.kanji_banks().len());
    info!("  - {} kanji meta banks", parser.kanji_meta_banks().len());

    let mut tx = db.begin().await.context("failed to start transaction")?;

    let result = sqlx::query!(
        "INSERT INTO dictionaries (title, revision) VALUES ($1, $2)",
        index.title,
        index.revision
    )
    .execute(&mut *tx)
    .await
    .context("failed to insert dictionary")?;
    let dictionary_id = DictionaryId(result.last_insert_rowid());

    let tx = Arc::new(Mutex::new(tx));
    let tasks = Mutex::new(JoinSet::<Result<()>>::new());
    let runtime = runtime::Handle::current();

    parser
        .run(
            |_, _| {},
            |_, bank| {
                tasks
                    .blocking_lock()
                    .spawn_on(import_term_bank(dictionary_id, tx.clone(), bank), &runtime);
                let term_banks_left = term_banks_left.fetch_sub(1, atomic::Ordering::SeqCst);
                info!("{term_banks_left} term banks left");
            },
            |_, bank| {
                tasks.blocking_lock().spawn_on(
                    import_term_meta_bank(dictionary_id, tx.clone(), bank),
                    &runtime,
                );
            },
            |_, _| {},
            |_, _| {},
        )
        .context("failed to parse banks")?;

    info!("Parse complete, waiting for database tasks to complete");
    let mut tasks = tasks.into_inner();
    while let Some(result) = tasks.join_next().await {
        info!("{} tasks left", tasks.len());
        result
            .context("import task cancelled")?
            .context("failed to import bank")?;
    }

    Arc::into_inner(tx)
        .expect("we own the last `Arc` after all tasks have been joined")
        .into_inner()
        .commit()
        .await
        .context("failed to commit transaction to database")?;

    let count: i64 = sqlx::query_scalar!("SELECT COUNT(*) FROM terms")
        .fetch_one(db)
        .await?;
    info!("Num expressions: {count:?}");

    Ok(())
}

async fn import_term_bank(
    dictionary_id: DictionaryId,
    tx: Arc<Mutex<Transaction<'_, Sqlite>>>,
    bank: TermBank,
) -> Result<()> {
    for term in bank {
        let expression = term.expression.clone();
        let reading = term.reading.clone();
        sqlx::query!(
            "INSERT INTO terms (dictionary, expression, reading) VALUES ($1, $2, $3)",
            dictionary_id.0,
            expression,
            reading,
        )
        .execute(&mut **tx.lock().await)
        .await
        .with_context(|| format!("failed to insert term {expression:?} ({reading:?})"))?;
    }

    Ok(())
}

async fn import_term_meta_bank(
    dictionary_id: DictionaryId,
    tx: Arc<Mutex<Transaction<'_, Sqlite>>>,
    bank: TermMetaBank,
) -> Result<()> {
    for term_meta in bank {
        // let expression = term_meta.expression.clone();
        // let reading = term_meta.reading.clone();
        // sqlx::query!(
        //     "INSERT INTO terms (expression, reading) VALUES ($1, $2)",
        //     expression,
        //     reading,
        // )
        // .execute(&mut **tx.lock().await)
        // .await
        // .with_context(|| format!("failed to insert {expression:?} ({reading:?}) into database"))?;
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
fn expression_entries() -> Vec<ExpressionEntry> {
    vec![
        ExpressionEntry {
            reading: Reading::from_pairs([("協", "きょう"), ("力", "りょく")]),
            frequency_sets: vec![
                FrequencySet {
                    dictionary: "JPDB".into(),
                    frequencies: vec![
                        Frequency {
                            value: 954,
                            display_value: None,
                        },
                        Frequency {
                            value: 131_342,
                            display_value: Some("131342㋕".into()),
                        },
                    ],
                },
                FrequencySet {
                    dictionary: "VN Freq".into(),
                    frequencies: vec![Frequency {
                        value: 948,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Novels".into(),
                    frequencies: vec![Frequency {
                        value: 1377,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Anime & J-drama".into(),
                    frequencies: vec![Frequency {
                        value: 1042,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Youtube".into(),
                    frequencies: vec![Frequency {
                        value: 722,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Wikipedia".into(),
                    frequencies: vec![Frequency {
                        value: 705,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "BCCWJ".into(),
                    frequencies: vec![
                        Frequency {
                            value: 597,
                            display_value: None,
                        },
                        Frequency {
                            value: 1395,
                            display_value: None,
                        },
                    ],
                },
                FrequencySet {
                    dictionary: "CC100".into(),
                    frequencies: vec![Frequency {
                        value: 741,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Innocent Ranked".into(),
                    frequencies: vec![Frequency {
                        value: 2343,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Narou Freq".into(),
                    frequencies: vec![Frequency {
                        value: 845,
                        display_value: None,
                    }],
                },
            ],
            pitch_sets: vec![PitchSet {
                dictionary: "NHK".into(),
                pitches: vec![Pitch { position: 1 }],
            }],
            glossary_sets: vec![
                GlossarySet {
                    dictionary: "Jitendex [2025-02-11]".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
                GlossarySet {
                    dictionary: "三省堂国語辞典　第八版".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
                GlossarySet {
                    dictionary: "明鏡国語辞典　第二版".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
                GlossarySet {
                    dictionary: "デジタル大辞泉".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
                GlossarySet {
                    dictionary: "PixivLight [2023-11-24]".into(),
                    glossaries: vec![Glossary {
                        todo: "TODO".into(),
                    }],
                },
            ],
        },
        ExpressionEntry {
            reading: Reading::from_no_pairs("協", ""),
            frequency_sets: vec![
                FrequencySet {
                    dictionary: "Novels".into(),
                    frequencies: vec![Frequency {
                        value: 29289,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Anime & J-drama".into(),
                    frequencies: vec![Frequency {
                        value: 26197,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Youtube".into(),
                    frequencies: vec![Frequency {
                        value: 23714,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Wikipedia".into(),
                    frequencies: vec![Frequency {
                        value: 6162,
                        display_value: None,
                    }],
                },
                FrequencySet {
                    dictionary: "Innocent Ranked".into(),
                    frequencies: vec![Frequency {
                        value: 18957,
                        display_value: None,
                    }],
                },
            ],
            pitch_sets: vec![],
            glossary_sets: vec![GlossarySet {
                dictionary: "JMnedict [2025-02-18]".into(),
                glossaries: vec![],
            }],
        },
    ]
}
