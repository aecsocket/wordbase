use core::arch;
use std::{
    convert::Infallible,
    io::Cursor,
    num::Wrapping,
    path::Path,
    sync::{
        Arc, Mutex,
        atomic::{self, AtomicUsize},
    },
};

use anyhow::{Context as _, Result, bail};
use derive_more::{Deref, DerefMut};
use futures::{SinkExt as _, StreamExt as _, never::Never};
use sqlx::{Pool, Sqlite, query, sqlite::SqlitePoolOptions};
use tokio::{
    fs,
    net::{TcpListener, TcpStream},
    sync::{broadcast, mpsc, oneshot},
};
use tokio_tungstenite::{WebSocketStream, tungstenite::Message};
use tracing::{Instrument, info, info_span, warn};
use wordbase::{
    dict::{
        ExpressionEntry, Frequency, FrequencySet, Glossary, GlossarySet, Pitch, PitchSet, Reading,
    },
    lookup::LookupInfo,
    protocol::{ClientRequest, FromClient, FromServer, Lookup, NewSentence, Response},
    yomitan::{self, TermBank},
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
    let db = SqlitePoolOptions::new()
        .max_connections(8)
        .connect("sqlite::memory:")
        .await
        .context("failed to connect to database")?;
    info!("Connected to SQLite database");

    sqlx::query(
        "CREATE TABLE expressions (
            expression TEXT PRIMARY KEY,
            reading TEXT NOT NULL
        )",
    )
    .execute(&db)
    .await
    .context("failed to set up database")?;

    let path = Path::new("/home/dev/dictionaries/jitendex.zip");
    todo_import(&db, &path)
        .instrument(info_span!("import", ?path))
        .await;

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
    db: Pool<Sqlite>,
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
                    warn!("Failed to handle request: {err:?}");
                    _ = connection.write(&FromServer::Error(format!("{err:?}"))).await;
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
    if message.is_empty() {
        return Ok(());
    }
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
        ClientRequest::AddAnkiNote(_) => {
            todo!();
        }
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
    let request_len = request.text.chars().count();
    let max_request_len = config.lookup.max_request_len;
    let request_len_valid =
        u16::try_from(request_len).is_ok_and(|request_len| request_len <= max_request_len);
    if !request_len_valid {
        bail!("request too long - {request_len} / {max_request_len} characters");
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
        lemma: mecab_response.lemma,
        expressions: expression_entries(), // TODO
    }))
}

async fn todo_import(db: &Pool<Sqlite>, path: impl AsRef<Path>) -> Result<()> {
    info!("Reading archive into memory");
    let archive = fs::read(path)
        .await
        .context("failed to read file into memory")?;

    let (parser, index) = yomitan::Parse::new(|| Ok::<_, Infallible>(Cursor::new(&archive)))
        .context("failed to parse")?;
    let term_banks_total = parser.term_banks().len();

    info!("{} rev {}:", index.title, index.revision);
    info!("  - {} tag banks", parser.tag_banks().len());
    info!("  - {term_banks_total} term banks");
    info!("  - {} term meta banks", parser.term_meta_banks().len());
    info!("  - {} kanji banks", parser.kanji_banks().len());
    info!("  - {} kanji meta banks", parser.kanji_meta_banks().len());

    let term_bank = Mutex::new(TermBank::default());
    let term_banks_left = AtomicUsize::new(term_banks_total);

    let tx = db
        .begin()
        .await
        .context("failed to start database transaction")?;
    // let tx = Mutex::new(tx);

    parser
        .run(
            |_, _| {},
            |_, new_term_bank| {
                {
                    let mut term_bank = term_bank.lock().expect("poisoned");
                    term_bank.extend_from_slice(&new_term_bank);
                }

                let term_banks_left = term_banks_left.fetch_sub(1, atomic::Ordering::SeqCst);
                info!("{term_banks_left} term banks left");
            },
            |_, _| {},
            |_, _| {},
            |_, _| {},
        )
        .context("failed to parse banks")?;

    for term in term_bank.into_inner().expect("poisoned") {
        let expression = term.expression;
        let reading = term.reading;
        sqlx::query("INSERT INTO expressions VALUES ($1, $2)")
            .bind(&expression)
            .bind(&reading)
            .execute(&mut *tx)
            .await
            .with_context(|| {
                format!("failed to insert {expression:?} ({reading:?}) into database")
            })?;
    }

    tx.commit()
        .await
        .context("failed to commit transaction to database")?;
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
