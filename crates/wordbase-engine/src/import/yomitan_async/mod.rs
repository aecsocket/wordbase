mod parse;
mod schema;

use {
    super::{ImportContinue, ImportKind, ImportStarted, insert_record, insert_term_record},
    crate::{CHANNEL_BUF_CAP, Engine, import::insert_dictionary},
    anyhow::{Context, Result, bail},
    async_zip::base::read::seek::ZipFileReader,
    bytes::Bytes,
    futures::{future::BoxFuture, io::Cursor},
    parse::ParseBanks,
    sqlx::{Sqlite, Transaction},
    std::{
        iter,
        sync::{
            Arc,
            atomic::{self, AtomicUsize},
        },
    },
    tokio::sync::mpsc,
    tracing::debug,
    wordbase::{
        DictionaryId, DictionaryKind, DictionaryMeta, NonEmptyString, Term,
        dict::yomitan::{Frequency, Glossary, GlossaryTag, Pitch, structured},
    },
};

pub struct Yomitan;

impl ImportKind for Yomitan {
    fn is_of_kind(&self, archive: Bytes) -> BoxFuture<'_, Result<()>> {
        Box::pin(validate(archive))
    }

    fn start_import<'a>(
        &'a self,
        engine: &'a Engine,
        archive: Bytes,
    ) -> BoxFuture<'a, Result<(ImportStarted, ImportContinue<'a>)>> {
        Box::pin(async move {
            let (result, continuation) = start_import(engine, archive).await?;
            Ok((result, Box::pin(continuation) as ImportContinue))
        })
    }
}

const INDEX_PATH: &str = "index.json";

async fn validate(archive: Bytes) -> Result<()> {
    let archive = ZipFileReader::new(Cursor::new(&archive))
        .await
        .context("failed to open zip archive")?;
    archive
        .file()
        .entries()
        .iter()
        .find(|entry| {
            entry
                .filename()
                .as_str()
                .is_ok_and(|name| name == INDEX_PATH)
        })
        .with_context(|| format!("no `{INDEX_PATH}` in archive"))?;
    Ok(())
}

async fn start_import(
    engine: &Engine,
    archive: Bytes,
) -> Result<(ImportStarted, impl Future<Output = Result<DictionaryId>>)> {
    let archive = ZipFileReader::new(Cursor::new(&archive)).await.context("");

    let archive = (&archive[..])
        .read_zip()
        .await
        .context("failed to open zip archive")?;

    let (parse_banks, index) = blocking::unblock(|| parse::start_blocking(archive))
        .await
        .context("failed to parse index")?;
    let mut meta = DictionaryMeta::new(DictionaryKind::Yomitan, index.title, index.revision);
    meta.description = index.description;
    meta.url = index.url;
    meta.attribution = index.attribution;

    let (send_progress, recv_progress) = mpsc::channel(CHANNEL_BUF_CAP);
    Ok((
        ImportStarted {
            meta: meta.clone(),
            recv_progress,
        },
        continue_import(engine, parse_banks, meta, send_progress),
    ))
}

async fn continue_import(
    engine: &Engine,
    parse_banks: ParseBanks,
    meta: DictionaryMeta,
    send_progress: mpsc::Sender<f64>,
) -> Result<DictionaryId> {
    let banks_len = parse_banks.tag_banks().len()
        + parse_banks.term_banks().len()
        + parse_banks.term_meta_banks().len()
        + parse_banks.kanji_banks().len()
        + parse_banks.kanji_meta_banks().len();
    let banks_done = Arc::new(AtomicUsize::new(0));

    let (send_bank_done, mut recv_bank_done) = mpsc::channel(CHANNEL_BUF_CAP);
    let parse_task = blocking::unblock({
        let banks_done = banks_done.clone();
        move || parse_banks.parse_blocking(&banks_done, &send_bank_done)
    });
    let forward_bank_done_task = async {
        while recv_bank_done.recv().await == Some(()) {
            let banks_done = banks_done.load(atomic::Ordering::SeqCst);
            let frac_done = 0.5 * ((banks_done as f64) / (banks_len as f64));
            _ = send_progress.try_send(frac_done);
        }
    };

    let (banks, ()) = tokio::join!(parse_task, forward_bank_done_task);
    let banks = banks.context("failed to parse banks")?;

    // explicitly exclude tags, since we don't insert tags as a record
    let records_len =
        banks.term.len() + banks.term_meta.len() + banks.kanji.len() + banks.kanji_meta.len();
    if records_len == 0 {
        bail!("no records to insert");
    }

    let records_done = AtomicUsize::new(0);
    debug!("Parse complete, inserting {records_len} records");

    debug!("Waiting for insert lock");
    let _tx_lock = engine.imports.insert_lock.lock().await;
    debug!("Lock acquired, starting transaction");
    let mut tx = engine
        .db
        .begin()
        .await
        .context("failed to begin transaction")?;
    debug!("Started transaction");

    let dictionary_id = insert_dictionary(&mut tx, &meta)
        .await
        .context("failed to insert dictionary")?;
    debug!("Inserted with ID {dictionary_id:?}");

    let mut all_tags = banks.tag.into_iter().map(to_term_tag).collect::<Vec<_>>();
    all_tags.sort_by(|tag_a, tag_b| tag_b.name.len().cmp(&tag_a.name.len()));

    debug!("Importing records");
    let notify_inserted = || {
        let records_done = records_done.fetch_add(1, atomic::Ordering::SeqCst) + 1;
        if records_done % 1000 == 0 {
            let frac_done = ((records_done as f64) / (records_len as f64)).mul_add(0.5, 0.5);
            _ = send_progress.try_send(frac_done);
        }
    };
    let mut scratch = Vec::<u8>::new();
    for term in banks.term {
        let headword = term.expression.clone();
        let reading = term.reading.clone();
        import_term(dictionary_id, &mut tx, term, &all_tags, &mut scratch)
            .await
            .with_context(|| format!("failed to import term ({headword:?}, {reading:?})"))?;
        notify_inserted();
    }
    for term_meta in banks.term_meta {
        let headword = term_meta.expression.clone();
        import_term_meta(dictionary_id, &mut tx, term_meta, &mut scratch)
            .await
            .with_context(|| format!("failed to import term meta {headword:?}"))?;
        notify_inserted();
    }
    drop(send_progress);

    tx.commit().await.context("failed to commit transaction")?;
    Ok(dictionary_id)
}

async fn import_term(
    source: DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    term_data: schema::Term,
    all_tags: &[GlossaryTag],
    scratch: &mut Vec<u8>,
) -> Result<()> {
    let term = Term::new(term_data.expression, term_data.reading)
        .context("term does not contain headword or reading")?;

    let tags = match_tags(
        all_tags,
        term_data.definition_tags.as_deref().unwrap_or_default(),
    )
    .cloned()
    .collect::<Vec<_>>();
    let content = term_data
        .glossary
        .into_iter()
        .filter_map(to_content)
        .collect();
    let record = Glossary {
        popularity: term_data.score,
        tags,
        content,
    };
    let record_id = insert_record(tx, source, &record, scratch)
        .await
        .context("failed to insert record")?;
    insert_term_record(tx, &term, record_id)
        .await
        .context("failed to insert term record")?;
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
    let headword = NonEmptyString::new(term_meta.expression);
    match term_meta.data {
        schema::TermMetaData::Frequency(frequency) => {
            let (record, reading) = to_frequency_and_reading(frequency);
            let term = Term::new(headword, reading)
                .context("frequency term has no headword or reading")?;

            let record_id = insert_record(tx, source, &record, scratch)
                .await
                .context("failed to insert frequency record")?;
            insert_term_record(tx, &term, record_id)
                .await
                .context("failed to insert frequency term record")?;
        }
        schema::TermMetaData::Pitch(pitch) => {
            for (record, reading) in to_pitches_and_readings(pitch) {
                let term = Term::new(headword.clone(), reading)
                    .context("pitch term has no headword or reading")?;

                let record_id = insert_record(tx, source, &record, scratch)
                    .await
                    .context("failed to insert pitch record")?;
                insert_term_record(tx, &term, record_id)
                    .await
                    .context("failed to insert pitch term record")?;
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

fn to_frequency_and_reading(raw: schema::TermMetaFrequency) -> (Frequency, Option<String>) {
    let (reading, generic) = match raw {
        schema::TermMetaFrequency::Generic(generic) => (None, generic),
        schema::TermMetaFrequency::WithReading { reading, frequency } => (Some(reading), frequency),
    };

    let frequency = match generic {
        schema::GenericFrequencyData::Number(rank) => Frequency {
            rank: Some(rank),
            display: None,
        },
        schema::GenericFrequencyData::String(rank) => rank.trim().parse().map_or(
            Frequency {
                rank: None,
                display: Some(rank),
            },
            |rank| Frequency {
                rank: Some(rank),
                display: None,
            },
        ),
        schema::GenericFrequencyData::Complex {
            value,
            display_value,
        } => Frequency {
            rank: Some(value),
            display: display_value,
        },
    };
    (frequency, reading)
}

fn to_pitches_and_readings(raw: schema::TermMetaPitch) -> impl Iterator<Item = (Pitch, String)> {
    raw.pitches.into_iter().map(move |pitch| {
        (
            Pitch {
                position: pitch.position,
                nasal: to_pitch_positions(pitch.nasal),
                devoice: to_pitch_positions(pitch.devoice),
            },
            raw.reading.clone(),
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
