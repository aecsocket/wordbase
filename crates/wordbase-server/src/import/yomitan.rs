use {
    super::{AlreadyExists, ReadToMemory},
    crate::{
        CHANNEL_BUF_CAP, dictionary,
        import::{Parsed, ReadMeta},
    },
    anyhow::{Context as _, Result},
    futures::TryFutureExt,
    sqlx::{Pool, Sqlite, Transaction},
    std::{
        convert::Infallible,
        io::Cursor,
        iter,
        path::Path,
        sync::atomic::{self, AtomicUsize},
    },
    tokio::{
        fs,
        sync::{Mutex, mpsc, oneshot},
    },
    tracing::{debug, info},
    wordbase::{
        Dictionary, DictionaryId,
        format::yomitan::{self, schema},
        lang::jp,
    },
};

pub async fn yomitan(
    db: Pool<Sqlite>,
    path: impl AsRef<Path>,
    send_read_to_memory: oneshot::Sender<ReadToMemory>,
) -> Result<Result<(), AlreadyExists>> {
    let path = path.as_ref();
    let archive = fs::read(path)
        .await
        .context("failed to read file into memory")?;

    let (send_read_meta, recv_read_meta) = oneshot::channel();
    _ = send_read_to_memory.send(ReadToMemory { recv_read_meta });

    let (parser, index) = yomitan::Parse::new(|| Ok::<_, Infallible>(Cursor::new(&archive)))
        .context("failed to parse")?;
    let meta = Dictionary {
        name: index.title,
        version: index.revision,
        ..Default::default()
    };
    let banks_len = parser.tag_banks().len()
        + parser.term_banks().len()
        + parser.term_meta_banks().len()
        + parser.kanji_banks().len()
        + parser.kanji_meta_banks().len();
    let banks_left = AtomicUsize::new(banks_len);

    debug!(
        "{:?} version {:?} - {banks_len} items",
        meta.name, meta.version
    );
    let (send_items_left, recv_items_left) = mpsc::channel(CHANNEL_BUF_CAP);
    let (send_parsed, recv_parsed) = oneshot::channel();
    _ = send_read_meta.send(ReadMeta {
        meta: meta.clone(),
        banks_len,
        recv_items_left,
        recv_parsed,
    });

    let already_exists = dictionary::exists_by_name(&db, &meta.name)
        .await
        .context("failed to check if dictionary exists")?;
    if already_exists {
        debug!("Dictionary already exists");
        return Ok(Err(AlreadyExists));
    }

    let tag_bank = Mutex::new(schema::TagBank::default());
    let term_bank = Mutex::new(schema::TermBank::default());
    let term_meta_bank = Mutex::new(schema::TermMetaBank::default());
    let kanji_bank = Mutex::new(schema::KanjiBank::default());
    let kanji_meta_bank = Mutex::new(schema::KanjiMetaBank::default());

    let notify_parsed = || {
        let items_left = banks_left.fetch_sub(1, atomic::Ordering::SeqCst) - 1;
        _ = send_items_left.try_send(items_left);
    };
    parser
        .run(
            |_, bank| {
                tag_bank.blocking_lock().extend_from_slice(&bank);
                notify_parsed();
            },
            |_, bank| {
                term_bank.blocking_lock().extend_from_slice(&bank);
                notify_parsed();
            },
            |_, bank| {
                term_meta_bank.blocking_lock().extend_from_slice(&bank);
                notify_parsed();
            },
            |_, _bank| {
                notify_parsed();
            },
            |_, _bank| {
                notify_parsed();
            },
        )
        .context("failed to parse banks")?;
    drop(send_items_left);
    let tag_bank = tag_bank.into_inner();
    let term_bank = term_bank.into_inner();
    let term_meta_bank = term_meta_bank.into_inner();
    let kanji_bank = kanji_bank.into_inner();
    let kanji_meta_bank = kanji_meta_bank.into_inner();

    let records_len =
        term_bank.len() + term_meta_bank.len() + kanji_bank.len() + kanji_meta_bank.len();
    let records_left = AtomicUsize::new(records_len);

    debug!("Parse complete, inserting {records_len} records");
    let (send_records_left, recv_records_left) = mpsc::channel(CHANNEL_BUF_CAP);
    let (send_inserted, recv_inserted) = oneshot::channel();
    _ = send_parsed.send(Parsed {
        records_len,
        recv_records_left,
        recv_inserted,
    });

    let mut tx = db.begin().await.context("failed to begin transaction")?;

    let dictionary_id = dictionary::insert(&mut *tx, &meta)
        .await
        .context("failed to insert dictionary")?;
    debug!("Inserted with ID {dictionary_id:?}");

    let mut all_tags = tag_bank.into_iter().map(to_term_tag).collect::<Vec<_>>();
    all_tags.sort_by(|tag_a, tag_b| tag_b.name.len().cmp(&tag_a.name.len()));

    debug!("Importing records");
    let notify_inserted = || {
        let records_left = records_left.fetch_sub(1, atomic::Ordering::SeqCst) - 1;
        _ = send_records_left.try_send(records_left);
    };
    tokio::try_join!(
        import_term_bank(
            dictionary_id,
            &mut tx,
            term_bank,
            notify_inserted,
            &all_tags,
        )
        .map_err(|err| err.context("failed to import term bank")),
        import_term_meta_bank(dictionary_id, &mut tx, term_meta_bank, notify_inserted)
            .map_err(|err| err.context("failed to import term meta bank"))
    )?;
    drop(send_records_left);

    _ = send_inserted.send(());
    tx.commit().await.context("failed to commit transaction")?;
    Ok(Ok(()))
}

fn none_if_empty(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

async fn import_term_bank(
    DictionaryId(source): DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    bank: schema::TermBank,
    notify_inserted: impl Fn(),
    all_tags: &[GlossaryTag],
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    let mut records_left = bank.len();
    for term in bank {
        let headword = term.expression.clone();
        let reading = none_if_empty(term.reading.clone());

        let tags = match_tags(
            all_tags,
            term.definition_tags.as_deref().unwrap_or_default(),
        )
        .cloned()
        .collect::<Vec<_>>();

        let mut glossary = Glossary::default();
        glossary.tags = tags;
        glossary.html = Some(to_html(term.glossary.into_iter()));

        async {
            scratch.clear();
            serialize(&glossary, &mut scratch).context("failed to serialize data")?;
            let data = &scratch[..];

            sqlx::query!(
                "INSERT INTO terms (source, headword, reading, data_kind, data)
                VALUES ($1, $2, $3, $4, $5)",
                source,
                headword,
                reading,
                data_kind::GLOSSARY,
                data
            )
            .execute(&mut **tx)
            .await
            .context("failed to insert record")?;

            records_left -= 1;
            if records_left % 10000 == 0 {
                info!("IMPORT: {records_left} term records left");
            }

            anyhow::Ok(())
        }
        .await
        .with_context(|| format!("failed to import term {headword:?} ({reading:?})"))?;
    }
    Ok(())
}

// `all_tags` must be sorted longest-first
fn match_tags<'a>(
    all_tags: &'a [GlossaryTag],
    mut definition_tags: &str,
) -> impl Iterator<Item = &'a GlossaryTag> {
    iter::from_fn(move || {
        definition_tags = definition_tags.trim();
        for tag in all_tags {
            if let Some(stripped) = definition_tags.strip_prefix(&tag.name) {
                definition_tags = stripped;
                return Some(tag);
            }
        }
        None
    })
}

async fn import_term_meta_bank(
    DictionaryId(source): DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    bank: schema::TermMetaBank,
    notify_inserted: impl Fn(),
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    let mut records_left = bank.len();
    for term_meta in bank {
        let headword = term_meta.expression.clone();
        async {
            match term_meta.data {
                schema::TermMetaData::Frequency(frequency) => {
                    for (reading, frequency) in to_frequencies(frequency) {
                        scratch.clear();
                        serialize(&frequency, &mut scratch).context("failed to serialize data")?;
                        let data = &scratch[..];

                        sqlx::query!(
                            "INSERT INTO terms (source, headword, reading, data_kind, data)
                            VALUES ($1, $2, $3, $4, $5)",
                            source,
                            headword,
                            reading,
                            data_kind::FREQUENCY,
                            data,
                        )
                        .execute(&mut **tx)
                        .await
                        .context("failed to insert record")?;
                    }
                }
                schema::TermMetaData::Pitch(pitch) => {
                    for (reading, pitch) in to_pitch(pitch) {
                        scratch.clear();
                        serialize(&pitch, &mut scratch).context("failed to serialize data")?;
                        let data = &scratch[..];

                        sqlx::query!(
                            "INSERT INTO terms (source, headword, reading, data_kind, data)
                            VALUES ($1, $2, $3, $4, $5)",
                            source,
                            headword,
                            reading,
                            data_kind::JP_PITCH,
                            data,
                        )
                        .execute(&mut **tx)
                        .await
                        .context("failed to insert record")?;
                    }
                }
                schema::TermMetaData::Phonetic(_) => {}
            }

            notify_inserted();
            anyhow::Ok(())
        }
        .await
        .with_context(|| format!("failed to import term meta {headword:?}"))?;
    }
    Ok(())
}

fn to_term_tag(raw: yomitan::Tag) -> GlossaryTag {
    GlossaryTag {
        name: raw.name,
        category: match raw.category.as_str() {
            "name" => Some(TagCategory::Name),
            "expression" => Some(TagCategory::Expression),
            "popular" => Some(TagCategory::Popular),
            "frequent" => Some(TagCategory::Frequent),
            "archaism" => Some(TagCategory::Archaism),
            "dictionary" => Some(TagCategory::Dictionary),
            "frequency" => Some(TagCategory::Frequency),
            "partOfSpeech" => Some(TagCategory::PartOfSpeech),
            "search" => Some(TagCategory::Search),
            "pronunciation-dictionary" => Some(TagCategory::PronunciationDictionary),
            _ => None,
        },
        description: raw.notes,
        order: raw.order,
    }
}

fn to_html(raw: impl Iterator<Item = yomitan::Glossary>) -> String {
    let mut html = String::new();
    for glossary in raw {
        if let Some(content) = to_content(glossary) {
            _ = yomitan::html::render_to_writer(&mut html, &content);
        }
    }
    html
}

fn to_content(raw: yomitan::Glossary) -> Option<structured::Content> {
    match raw {
        yomitan::Glossary::Deinflection(_) => None,
        yomitan::Glossary::String(text)
        | yomitan::Glossary::Content(yomitan::GlossaryContent::Text { text }) => {
            Some(structured::Content::String(text))
        }
        yomitan::Glossary::Content(yomitan::GlossaryContent::Image(base)) => {
            Some(structured::Content::Element(Box::new(
                structured::Element::Img(structured::ImageElement {
                    base,
                    ..Default::default()
                }),
            )))
        }
        yomitan::Glossary::Content(yomitan::GlossaryContent::StructuredContent { content }) => {
            Some(content)
        }
    }
}

fn to_frequencies(
    raw: yomitan::TermMetaFrequency,
) -> impl Iterator<Item = (Option<String>, Frequency)> {
    let (reading, generic) = match raw {
        yomitan::TermMetaFrequency::Generic(generic) => (None, generic),
        yomitan::TermMetaFrequency::WithReading { reading, frequency } => {
            (Some(reading), frequency)
        }
    };

    let frequency = match generic {
        yomitan::GenericFrequencyData::Number(rank) => Some(Frequency::new(rank)),
        yomitan::GenericFrequencyData::String(rank) => {
            // best-effort attempt
            rank.trim().parse::<u64>().map(Frequency::new).ok()
        }
        yomitan::GenericFrequencyData::Complex {
            value: rank,
            display_value: display_rank,
        } => Some(Frequency {
            rank,
            display: display_rank,
        }),
    };

    frequency.map(|new| (reading, new)).into_iter()
}

fn to_pitch(raw: yomitan::TermMetaPitch) -> impl Iterator<Item = (String, jp::Pitch)> {
    raw.pitches.into_iter().map(move |pitch| {
        (
            raw.reading.clone(),
            jp::Pitch {
                position: pitch.position,
                nasal: to_pitch_positions(pitch.nasal),
                devoice: to_pitch_positions(pitch.devoice),
            },
        )
    })
}

fn to_pitch_positions(raw: Option<yomitan::PitchPosition>) -> Vec<u64> {
    match raw {
        None => vec![],
        Some(yomitan::PitchPosition::One(position)) => vec![position],
        Some(yomitan::PitchPosition::Many(positions)) => positions,
    }
}
