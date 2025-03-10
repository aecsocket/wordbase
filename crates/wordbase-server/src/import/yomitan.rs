use {
    super::{AlreadyExists, ReadToMemory},
    crate::{
        CHANNEL_BUF_CAP, dictionary,
        import::{Parsed, ReadMeta},
        term,
    },
    anyhow::{Context as _, Result},
    sqlx::{Pool, Sqlite, Transaction},
    std::{
        convert::Infallible,
        io::Cursor,
        iter,
        path::Path,
        sync::{
            Arc,
            atomic::{self, AtomicUsize},
        },
    },
    tokio::{
        fs,
        sync::{Mutex, Semaphore, mpsc, oneshot},
    },
    tracing::debug,
    wordbase::{
        DictionaryId, DictionaryMeta, Term,
        format::{
            self,
            yomitan::{self, GlossaryTag, schema, structured},
        },
        lang,
        record::Frequency,
    },
};

pub async fn yomitan(
    db: Pool<Sqlite>,
    import_semaphore: Arc<Semaphore>,
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
    let meta = DictionaryMeta {
        name: index.title,
        version: index.revision,
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
    let (send_banks_left, recv_banks_left) = mpsc::channel(CHANNEL_BUF_CAP);
    let (send_parsed, recv_parsed) = oneshot::channel();
    _ = send_read_meta.send(ReadMeta {
        meta: meta.clone(),
        banks_len,
        recv_banks_left,
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
        _ = send_banks_left.try_send(items_left);
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
    drop(send_banks_left);
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

    debug!("Waiting for permit to start transaction");
    let _import_permit = import_semaphore
        .acquire()
        .await
        .context("import permit closed")?;
    debug!("Permit acquired, starting transaction");
    let mut tx = db.begin().await.context("failed to begin transaction")?;
    debug!("Started transaction");

    let dictionary_id = dictionary::insert(&mut *tx, &meta)
        .await
        .context("failed to insert dictionary")?;
    debug!("Inserted with ID {dictionary_id:?}");

    let mut all_tags = tag_bank.into_iter().map(to_term_tag).collect::<Vec<_>>();
    all_tags.sort_by(|tag_a, tag_b| tag_b.name.len().cmp(&tag_a.name.len()));

    debug!("Importing records");
    let notify_inserted = || {
        let records_left = records_left.fetch_sub(1, atomic::Ordering::SeqCst) - 1;
        if records_left % 1000 == 0 {
            _ = send_records_left.try_send(records_left);
        }
    };
    let mut scratch = Vec::<u8>::new();
    for term in term_bank {
        let headword = term.expression.clone();
        let reading = term.reading.clone();
        import_term(dictionary_id, &mut tx, term, &all_tags, &mut scratch)
            .await
            .with_context(|| format!("failed to import term ({headword:?}, {reading:?})"))?;
        notify_inserted();
    }
    for term_meta in term_meta_bank {
        let headword = term_meta.expression.clone();
        import_term_meta(dictionary_id, &mut tx, term_meta, &mut scratch)
            .await
            .with_context(|| format!("failed to import term meta {headword:?}"))?;
        notify_inserted();
    }
    drop(send_records_left);

    _ = send_inserted.send(());
    tx.commit().await.context("failed to commit transaction")?;
    Ok(Ok(()))
}

fn none_if_empty(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

async fn import_term(
    source: DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    term: schema::Term,
    all_tags: &[GlossaryTag],
    scratch: &mut Vec<u8>,
) -> Result<()> {
    let headword = term.expression.clone();
    let reading = none_if_empty(term.reading.clone());

    let tags = match_tags(
        all_tags,
        term.definition_tags.as_deref().unwrap_or_default(),
    )
    .cloned()
    .collect::<Vec<_>>();
    let content = term.glossary.into_iter().filter_map(to_content).collect();
    let glossary = format::yomitan::Glossary { tags, content };

    term::insert(
        &mut **tx,
        source,
        &Term { headword, reading },
        &glossary,
        scratch,
    )
    .await?;
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

async fn import_term_meta(
    source: DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    term_meta: schema::TermMeta,
    scratch: &mut Vec<u8>,
) -> Result<()> {
    let headword = term_meta.expression;
    match term_meta.data {
        schema::TermMetaData::Frequency(frequency) => {
            for (reading, record) in to_frequencies(frequency) {
                term::insert(
                    &mut **tx,
                    source,
                    &Term {
                        headword: headword.clone(),
                        reading,
                    },
                    &record,
                    scratch,
                )
                .await
                .context("failed to insert frequency record")?;
            }
        }
        schema::TermMetaData::Pitch(pitch) => {
            for (reading, record) in to_pitch(pitch) {
                term::insert(
                    &mut **tx,
                    source,
                    &Term::with_reading(headword.clone(), reading),
                    &record,
                    scratch,
                )
                .await
                .context("failed to insert pitch data")?;
            }
        }
        schema::TermMetaData::Phonetic(_) => {}
    }
    Ok(())
}

fn to_term_tag(raw: schema::Tag) -> GlossaryTag {
    GlossaryTag {
        name: raw.name,
        category: raw.category,
        description: raw.notes,
        order: raw.order,
    }
}

fn to_content(raw: schema::Glossary) -> Option<structured::Content> {
    match raw {
        schema::Glossary::Deinflection(_) => None,
        schema::Glossary::String(text)
        | schema::Glossary::Content(schema::GlossaryContent::Text { text }) => {
            Some(structured::Content::String(text))
        }
        schema::Glossary::Content(schema::GlossaryContent::Image(base)) => {
            Some(structured::Content::Element(Box::new(
                structured::Element::Img(structured::ImageElement {
                    base,
                    ..Default::default()
                }),
            )))
        }
        schema::Glossary::Content(schema::GlossaryContent::StructuredContent { content }) => {
            Some(content)
        }
    }
}

fn to_frequencies(
    raw: schema::TermMetaFrequency,
) -> impl Iterator<Item = (Option<String>, Frequency)> {
    let (reading, generic) = match raw {
        schema::TermMetaFrequency::Generic(generic) => (None, generic),
        schema::TermMetaFrequency::WithReading { reading, frequency } => (Some(reading), frequency),
    };

    let frequency = match generic {
        schema::GenericFrequencyData::Number(rank) => Some(Frequency::new(rank)),
        schema::GenericFrequencyData::String(rank) => {
            // best-effort attempt
            rank.trim().parse::<u64>().map(Frequency::new).ok()
        }
        schema::GenericFrequencyData::Complex {
            value: rank,
            display_value: display,
        } => Some(Frequency { rank, display }),
    };

    frequency.map(|new| (reading, new)).into_iter()
}

fn to_pitch(raw: schema::TermMetaPitch) -> impl Iterator<Item = (String, lang::jp::Pitch)> {
    raw.pitches.into_iter().map(move |pitch| {
        (
            raw.reading.clone(),
            lang::jp::Pitch {
                position: pitch.position,
                nasal: to_pitch_positions(pitch.nasal),
                devoice: to_pitch_positions(pitch.devoice),
            },
        )
    })
}

fn to_pitch_positions(raw: Option<schema::PitchPosition>) -> Vec<u64> {
    match raw {
        None => vec![],
        Some(schema::PitchPosition::One(position)) => vec![position],
        Some(schema::PitchPosition::Many(positions)) => positions,
    }
}
