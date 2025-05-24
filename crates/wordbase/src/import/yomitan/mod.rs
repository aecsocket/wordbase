mod schema;

use {
    super::{Archive, ImportContinue, ImportKind, OpenArchive},
    crate::import::{insert::Inserter, insert_dictionary},
    anyhow::{Context, Result},
    async_zip::base::read::seek::ZipFileReader,
    derive_more::From,
    futures::future::BoxFuture,
    schema::{
        FrequencyMode, INDEX_PATH, KANJI_BANK_PATTERN, KANJI_META_BANK_PATTERN, TAG_BANK_PATTERN,
        TERM_BANK_PATTERN, TERM_META_BANK_PATTERN,
    },
    serde::de::DeserializeOwned,
    sqlx::{Pool, Sqlite},
    std::{iter, sync::Arc},
    tokio::{
        sync::{Semaphore, mpsc},
        task::JoinSet,
    },
    tokio_util::compat::Compat,
    tracing::{debug, trace},
    wordbase_api::{
        DictionaryId, DictionaryKind, DictionaryMeta, FrequencyValue, NoHeadwordOrReading,
        NormString, Term,
        dict::{
            jpn::PitchPosition,
            yomitan::{Frequency, Glossary, GlossaryTag, Pitch, structured},
        },
    },
};

pub struct Yomitan;

impl ImportKind for Yomitan {
    fn is_of_kind(&self, open_archive: Arc<dyn OpenArchive>) -> BoxFuture<'_, Result<()>> {
        Box::pin(validate(open_archive))
    }

    fn start_import(
        &self,
        db: Pool<Sqlite>,
        open_archive: Arc<dyn OpenArchive>,
        progress_tx: mpsc::Sender<f64>,
    ) -> BoxFuture<Result<(DictionaryMeta, ImportContinue)>> {
        Box::pin(async move {
            let (meta, continuation) = start_import(db, open_archive, progress_tx).await?;
            Ok((meta, Box::pin(continuation) as ImportContinue))
        })
    }
}

async fn archive_reader(
    open_archive: &dyn OpenArchive,
) -> Result<ZipFileReader<Compat<Box<dyn Archive>>>> {
    let archive = open_archive
        .open_archive()
        .await
        .context("failed to open archive")?;
    let archive = ZipFileReader::with_tokio(archive)
        .await
        .context("failed to open zip archive")?;
    Ok(archive)
}

async fn validate(open_archive: Arc<dyn OpenArchive>) -> Result<()> {
    archive_reader(&*open_archive)
        .await?
        .file()
        .entries()
        .iter()
        .find(|entry| {
            let Ok(path) = entry.filename().as_str() else {
                return false;
            };
            path == INDEX_PATH
        })
        .with_context(|| format!("no `{INDEX_PATH}` in archive"))?;
    Ok(())
}

async fn start_import(
    db: Pool<Sqlite>,
    open_archive: Arc<dyn OpenArchive>,
    progress_tx: mpsc::Sender<f64>,
) -> Result<(DictionaryMeta, impl Future<Output = Result<DictionaryId>>)> {
    let mut archive = archive_reader(&*open_archive).await?;
    let index_index = archive
        .file()
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            entry
                .filename()
                .as_str()
                .is_ok_and(|name| name == INDEX_PATH)
        })
        .map(|(index, _)| index)
        .next()
        .with_context(|| format!("no `{INDEX_PATH}` in archive"))?;
    let mut index_entry = archive
        .reader_with_entry(index_index)
        .await
        .context("failed to start reading index")?;
    let mut index = Vec::new();
    index_entry
        .read_to_end_checked(&mut index)
        .await
        .context("failed to read index into memory")?;
    let index = serde_json::from_slice::<schema::Index>(&index).context("failed to parse index")?;

    let mut meta = DictionaryMeta::new(DictionaryKind::Yomitan, index.title.clone());
    meta.version = Some(index.revision.clone());
    index.description.clone_into(&mut meta.description);
    index.url.clone_into(&mut meta.url);
    index.attribution.clone_into(&mut meta.attribution);

    Ok((
        meta.clone(),
        continue_import(db, open_archive, meta, index, progress_tx),
    ))
}

// TODO: make this configurable somehow
const BANK_BUF_CAP: usize = 1;

async fn continue_import(
    db: Pool<Sqlite>,
    open_archive: Arc<dyn OpenArchive>,
    meta: DictionaryMeta,
    index: schema::Index,
    progress_tx: mpsc::Sender<f64>,
) -> Result<DictionaryId> {
    trace!("Importing Yomitan");

    // stage 1: read dictionary meta and find what banks we have
    let archive = archive_reader(&*open_archive).await?;

    let mut tag_bank_paths = Vec::<(usize, String)>::new();
    let mut term_bank_paths = Vec::<(usize, String)>::new();
    let mut term_meta_bank_paths = Vec::<(usize, String)>::new();
    let mut kanji_bank_paths = Vec::<(usize, String)>::new();
    let mut kanji_meta_bank_paths = Vec::<(usize, String)>::new();
    for (index, entry) in archive.file().entries().iter().enumerate() {
        let filename = entry.filename();
        let path = filename
            .as_str()
            .with_context(|| format!("`{filename:?}` is not a UTF-8 file name"))?
            .to_owned();

        if TAG_BANK_PATTERN.is_match(&path) {
            tag_bank_paths.push((index, path));
        } else if TERM_BANK_PATTERN.is_match(&path) {
            term_bank_paths.push((index, path));
        } else if TERM_META_BANK_PATTERN.is_match(&path) {
            term_meta_bank_paths.push((index, path));
        } else if KANJI_BANK_PATTERN.is_match(&path) {
            kanji_bank_paths.push((index, path));
        } else if KANJI_META_BANK_PATTERN.is_match(&path) {
            kanji_meta_bank_paths.push((index, path));
        }
    }
    let num_banks = term_bank_paths.len()
        + term_meta_bank_paths.len()
        + kanji_bank_paths.len()
        + kanji_meta_bank_paths.len();

    // stage 2: spawn tasks to parse banks
    // - parse a bank in another task
    // - send that bank to this thread, so we can insert it into the tx
    // - keep up to `BANK_BUF_CAP` banks parsed at once
    // - do NOT parse all banks first, then insert them in bulk -
    //   this will use a lot of memory! (Jitendex would use ~4 GB)
    let parse_permits = Arc::new(Semaphore::new(BANK_BUF_CAP));
    let (to_insert_tx, mut to_insert_rx) = mpsc::channel::<Bank>(BANK_BUF_CAP * 2);
    let mut tasks = JoinSet::new();

    spawn_parse_tasks::<schema::Term>(
        term_bank_paths,
        &open_archive,
        &parse_permits,
        &to_insert_tx,
        &mut tasks,
    );
    spawn_parse_tasks::<schema::TermMeta>(
        term_meta_bank_paths,
        &open_archive,
        &parse_permits,
        &to_insert_tx,
        &mut tasks,
    );
    spawn_parse_tasks::<schema::Kanji>(
        kanji_bank_paths,
        &open_archive,
        &parse_permits,
        &to_insert_tx,
        &mut tasks,
    );
    spawn_parse_tasks::<schema::KanjiMeta>(
        kanji_meta_bank_paths,
        &open_archive,
        &parse_permits,
        &to_insert_tx,
        &mut tasks,
    );
    drop(to_insert_tx);

    // stage 3: parse tag banks manually
    // since we need them for inserting term banks later
    let mut tag_bank = Vec::new();
    for (entry_index, entry_path) in tag_bank_paths {
        let mut bank = parse_bank::<schema::Tag>(&*open_archive, entry_index)
            .await
            .with_context(|| format!("failed to parse tag bank `{entry_path}`"))?;
        tag_bank.append(&mut bank);
    }
    let mut all_tags = tag_bank.into_iter().map(to_term_tag).collect::<Vec<_>>();
    all_tags.sort_by_key(|tag| tag.name.len());

    // stage 4: start inserting
    let mut tx = db.begin().await.context("failed to begin transaction")?;
    let dictionary_id = insert_dictionary(&mut tx, &meta)
        .await
        .context("failed to insert dictionary")?;
    let mut insert = Inserter::new(&mut tx, dictionary_id).await?;

    let notify_progress = |banks_done: usize, rows_done: usize, rows_len: usize| {
        // +1 to not trigger on the first row
        if (rows_done + 1) % 500 == 0 {
            let bank_progress = rows_done as f64 / rows_len as f64;
            let progress = (banks_done as f64 + bank_progress) / (num_banks as f64);
            _ = progress_tx.try_send(progress);
        }
    };

    let mut banks_done = 0usize;
    while let Some(to_insert) = to_insert_rx.recv().await {
        match to_insert {
            Bank::TermMeta(bank) => {
                let rows_len = bank.len();
                for (row_idx, term_meta) in bank.into_iter().enumerate() {
                    let headword = term_meta.expression.clone();
                    import_term_meta(&mut insert, &index, term_meta)
                        .await
                        .with_context(|| format!("failed to import term meta {headword:?}"))?;
                    notify_progress(banks_done, row_idx, rows_len);
                }
            }
            Bank::Term(bank) => {
                let rows_len = bank.len();
                for (row_idx, term) in bank.into_iter().enumerate() {
                    let headword = term.expression.clone();
                    let reading = term.reading.clone();
                    import_term(&mut insert, term, &all_tags)
                        .await
                        .with_context(|| {
                            format!("failed to import term ({headword:?}, {reading:?})")
                        })?;
                    notify_progress(banks_done, row_idx, rows_len);
                }
            }
            Bank::Kanji(bank) => {
                _ = bank;
            }
            Bank::KanjiMeta(bank) => {
                _ = bank;
            }
        }
        banks_done += 1;
        let progress = banks_done as f64 / num_banks as f64;
        _ = progress_tx.try_send(progress);
    }
    while let Some(res) = tasks.join_next().await {
        res.context("task canceled")??;
    }

    debug!("Insert complete, flushing");
    insert.flush().await.context("failed to flush inserts")?;
    tx.commit().await.context("failed to commit transaction")?;
    Ok(dictionary_id)
}

#[derive(Debug, From)]
enum Bank {
    Term(Vec<schema::Term>),
    TermMeta(Vec<schema::TermMeta>),
    Kanji(Vec<schema::Kanji>),
    KanjiMeta(Vec<schema::KanjiMeta>),
}

fn spawn_parse_tasks<T>(
    bank_entries: impl IntoIterator<Item = (usize, String)>,
    open_archive: &Arc<dyn OpenArchive>,
    parse_permits: &Arc<Semaphore>,
    to_insert_tx: &mpsc::Sender<Bank>,
    tasks: &mut JoinSet<Result<()>>,
) where
    Bank: From<Vec<T>>,
    Vec<T>: DeserializeOwned,
{
    for (entry_index, entry_path) in bank_entries {
        let open_archive = open_archive.clone();
        let parse_permits = parse_permits.clone();
        let to_insert_tx = to_insert_tx.clone();
        tasks.spawn(async move {
            let _permit = parse_permits.acquire().await?;
            let bank = parse_bank::<T>(&*open_archive, entry_index)
                .await
                .with_context(|| format!("failed to parse bank `{entry_path}`"))?;
            to_insert_tx.send(Bank::from(bank)).await?;
            anyhow::Ok(())
        });
    }
}

async fn parse_bank<T>(open_archive: &dyn OpenArchive, entry_index: usize) -> Result<Vec<T>>
where
    Vec<T>: DeserializeOwned,
{
    let mut archive = archive_reader(open_archive).await?;
    let mut entry = archive
        .reader_with_entry(entry_index)
        .await
        .context("failed to read entry")?;
    let mut bank_data = Vec::new();
    entry
        .read_to_end_checked(&mut bank_data)
        .await
        .context("failed to read bank into memory")?;
    let bank = serde_json::from_slice::<Vec<T>>(&bank_data).context("failed to parse bank")?;
    Ok(bank)
}

fn to_term_tag(raw: schema::Tag) -> GlossaryTag {
    GlossaryTag {
        name: raw.name,
        category: raw.category,
        description: raw.notes,
        order: raw.order,
    }
}

async fn import_term(
    insert: &mut Inserter<'_, '_>,
    term_data: schema::Term,
    all_tags: &[GlossaryTag],
) -> Result<()> {
    let term =
        Term::from_full(term_data.expression, term_data.reading).ok_or(NoHeadwordOrReading)?;

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

    let record_id = insert
        .record(&record)
        .await
        .context("failed to insert record")?;
    insert
        .term_record(term.clone(), record_id)
        .await
        .context("failed to insert term record")?;
    insert
        .frequency(term, FrequencyValue::Occurrence(term_data.score))
        .await
        .context("failed to insert frequency record")?;
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
    insert: &mut Inserter<'_, '_>,
    index: &schema::Index,
    term_meta: schema::TermMeta,
) -> Result<()> {
    let headword = NormString::new(term_meta.expression);
    match term_meta.data {
        schema::TermMetaData::Frequency(frequency) => {
            // Yomitan dictionaries like VN Freq v2 seem to default to rank-based
            let frequency_mode = index.frequency_mode.unwrap_or(FrequencyMode::RankBased);
            let (record, reading) = to_frequency_and_reading(frequency_mode, frequency);
            let term = Term::from_parts(headword, reading).ok_or(NoHeadwordOrReading)?;

            let record_id = insert
                .record(&record)
                .await
                .context("failed to insert record")?;
            insert
                .term_record(term.clone(), record_id)
                .await
                .context("failed to insert term record")?;

            if let Some(value) = record.value {
                insert
                    .frequency(term, value)
                    .await
                    .context("failed to insert frequency record")?;
            }
        }
        schema::TermMetaData::Pitch(pitch) => {
            for (record, reading) in to_pitches_and_readings(pitch) {
                let term =
                    Term::from_parts(headword.clone(), Some(reading)).ok_or(NoHeadwordOrReading)?;

                let record_id = insert
                    .record(&record)
                    .await
                    .context("failed to insert record")?;
                insert
                    .term_record(term, record_id)
                    .await
                    .context("failed to insert term record")?;
            }
        }
        schema::TermMetaData::Phonetic(_) => {}
    }
    Ok(())
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

fn to_frequency_and_reading(
    frequency_mode: schema::FrequencyMode,
    raw: schema::TermMetaFrequency,
) -> (Frequency, Option<String>) {
    let (reading, generic) = match raw {
        schema::TermMetaFrequency::Generic(generic) => (None, generic),
        schema::TermMetaFrequency::WithReading { reading, frequency } => (Some(reading), frequency),
    };

    let value_from = |n: i64| match frequency_mode {
        schema::FrequencyMode::OccurrenceBased => FrequencyValue::Occurrence(n),
        schema::FrequencyMode::RankBased => FrequencyValue::Rank(n),
    };

    let frequency = match generic {
        schema::GenericFrequencyData::Number(rank) => Frequency {
            value: Some(value_from(rank)),
            display: None,
        },
        schema::GenericFrequencyData::String(rank) => rank.trim().parse::<i64>().map_or(
            Frequency {
                value: None,
                display: Some(rank),
            },
            |rank| Frequency {
                value: Some(value_from(rank)),
                display: None,
            },
        ),
        schema::GenericFrequencyData::Complex {
            value,
            display_value,
        } => Frequency {
            value: Some(value_from(value)),
            display: display_value,
        },
    };
    (frequency, reading)
}

fn to_pitches_and_readings(raw: schema::TermMetaPitch) -> impl Iterator<Item = (Pitch, String)> {
    raw.pitches.into_iter().map(move |pitch| {
        (
            Pitch {
                position: PitchPosition(pitch.position),
                nasal: to_pitch_positions(pitch.nasal),
                devoice: to_pitch_positions(pitch.devoice),
            },
            raw.reading.clone(),
        )
    })
}

fn to_pitch_positions(raw: Option<schema::PitchPosition>) -> Vec<PitchPosition> {
    match raw {
        None => vec![],
        Some(schema::PitchPosition::One(position)) => vec![PitchPosition(position)],
        Some(schema::PitchPosition::Many(positions)) => {
            positions.into_iter().map(PitchPosition).collect()
        }
    }
}
