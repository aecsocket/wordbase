mod schema;

use {
    super::{ImportContinue, ImportKind},
    crate::import::{insert::Insert, insert_dictionary},
    anyhow::{Context, Result, bail},
    async_zip::base::read::seek::ZipFileReader,
    futures::future::BoxFuture,
    schema::{
        FrequencyMode, INDEX_PATH, KANJI_BANK_PATTERN, KANJI_META_BANK_PATTERN, TAG_BANK_PATTERN,
        TERM_BANK_PATTERN, TERM_META_BANK_PATTERN,
    },
    serde::de::DeserializeOwned,
    sqlx::{Pool, Sqlite, Transaction},
    std::{
        iter,
        path::Path,
        sync::{
            Arc,
            atomic::{self, AtomicUsize},
        },
    },
    tokio::{fs::File, io::BufReader, sync::mpsc, task::JoinSet},
    tokio_util::compat::Compat,
    tracing::{debug, trace},
    wordbase_api::{
        DictionaryId, DictionaryKind, DictionaryMeta, FrequencyValue, NormString, Record, Term,
        dict::{
            jpn::PitchPosition,
            yomitan::{Frequency, Glossary, GlossaryTag, Pitch, structured},
        },
    },
};

pub struct Yomitan;

impl ImportKind for Yomitan {
    fn is_of_kind(&self, archive_path: Arc<Path>) -> BoxFuture<'_, Result<()>> {
        Box::pin(validate(archive_path))
    }

    fn start_import(
        &self,
        db: Pool<Sqlite>,
        archive_path: Arc<Path>,
        progress_tx: mpsc::Sender<f64>,
    ) -> BoxFuture<Result<(DictionaryMeta, ImportContinue)>> {
        Box::pin(async move {
            let (meta, continuation) = start_import(db, archive_path, progress_tx).await?;
            Ok((meta, Box::pin(continuation) as ImportContinue))
        })
    }
}

async fn open_archive(archive_path: &Path) -> Result<ZipFileReader<Compat<BufReader<File>>>> {
    let file = File::open(archive_path)
        .await
        .context("failed to open file")?;
    let archive = ZipFileReader::with_tokio(BufReader::new(file))
        .await
        .context("failed to open zip archive")?;
    Ok(archive)
}

async fn validate(archive_path: Arc<Path>) -> Result<()> {
    open_archive(&archive_path)
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
    archive_path: Arc<Path>,
    progress_tx: mpsc::Sender<f64>,
) -> Result<(DictionaryMeta, impl Future<Output = Result<DictionaryId>>)> {
    let mut archive = open_archive(&archive_path).await?;
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
        continue_import(db, archive_path, meta, index, progress_tx),
    ))
}

async fn continue_import(
    db: Pool<Sqlite>,
    archive_path: Arc<Path>,
    meta: DictionaryMeta,
    index: schema::Index,
    progress_tx: mpsc::Sender<f64>,
) -> Result<DictionaryId> {
    trace!("Importing Yomitan");

    // stage 1: meta
    let archive = open_archive(&archive_path).await?;

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

    // stage 2: banks
    let num_banks = tag_bank_paths.len()
        + term_bank_paths.len()
        + term_meta_bank_paths.len()
        + kanji_bank_paths.len()
        + kanji_meta_bank_paths.len();
    let banks_parsed = Arc::new(AtomicUsize::new(0));
    let (tag_bank, term_bank, term_meta_bank, kanji_bank, kanji_meta_bank) = tokio::try_join!(
        spawn_flatten(parse_all_banks::<schema::Tag>(
            tag_bank_paths,
            archive_path.clone(),
            banks_parsed.clone(),
            num_banks,
            progress_tx.clone()
        )),
        spawn_flatten(parse_all_banks::<schema::Term>(
            term_bank_paths,
            archive_path.clone(),
            banks_parsed.clone(),
            num_banks,
            progress_tx.clone(),
        )),
        spawn_flatten(parse_all_banks::<schema::TermMeta>(
            term_meta_bank_paths,
            archive_path.clone(),
            banks_parsed.clone(),
            num_banks,
            progress_tx.clone(),
        )),
        spawn_flatten(parse_all_banks::<schema::Kanji>(
            kanji_bank_paths,
            archive_path.clone(),
            banks_parsed.clone(),
            num_banks,
            progress_tx.clone(),
        )),
        spawn_flatten(parse_all_banks::<schema::KanjiMeta>(
            kanji_meta_bank_paths,
            archive_path.clone(),
            banks_parsed.clone(),
            num_banks,
            progress_tx.clone(),
        )),
    )
    .context("failed to parse banks")?;

    // explicitly exclude tags, since we don't insert tags as a record
    let records_len =
        term_bank.len() + term_meta_bank.len() + kanji_bank.len() + kanji_meta_bank.len();
    if records_len == 0 {
        bail!("no records to insert");
    }

    let mut all_tags = tag_bank.into_iter().map(to_term_tag).collect::<Vec<_>>();
    all_tags.sort_by_key(|tag| tag.name.len());

    let records_done = AtomicUsize::new(0);
    let notify_inserted = || {
        let records_done = records_done.fetch_add(1, atomic::Ordering::SeqCst) + 1;
        if records_done % 2000 == 0 {
            let progress = (records_done as f64) / (records_len as f64);
            _ = progress_tx.try_send(progress.mul_add(0.5, 0.5));
        }
    };

    println!("tm = {} / tmb = {}", term_bank.len(), term_meta_bank.len());
    loop {}

    debug!("Importing records");
    let mut tx = db.begin().await.context("failed to begin transaction")?;
    let dictionary_id = insert_dictionary(&mut tx, &meta)
        .await
        .context("failed to insert dictionary")?;

    let mut records = Insert::<Record>::new();
    let mut frequencies = Insert::<FrequencyValue>::new();

    for term in term_bank {
        let headword = term.expression.clone();
        let reading = term.reading.clone();
        import_term(
            dictionary_id,
            &mut tx,
            &mut records,
            &mut frequencies,
            term,
            &all_tags,
        )
        .await
        .with_context(|| format!("failed to import term ({headword:?}, {reading:?})"))?;
        notify_inserted();
    }

    for term_meta in term_meta_bank {
        let headword = term_meta.expression.clone();
        import_term_meta(
            dictionary_id,
            &mut tx,
            &mut records,
            &mut frequencies,
            &index,
            term_meta,
        )
        .await
        .with_context(|| format!("failed to import term meta {headword:?}"))?;
        notify_inserted();
    }

    records
        .flush(&mut tx)
        .await
        .context("failed to flush records")?;
    frequencies
        .flush(&mut tx)
        .await
        .context("failed to flush frequencies")?;

    drop(progress_tx);

    tx.commit().await.context("failed to commit transaction")?;
    Ok(dictionary_id)
}

async fn spawn_flatten<F, T>(fut: F) -> Result<T>
where
    F: Future<Output = Result<T>> + Send + 'static,
    T: Send + 'static,
{
    match tokio::spawn(fut).await.context("task canceled") {
        Ok(Ok(t)) => Ok(t),
        Ok(Err(err)) | Err(err) => Err(err),
    }
}

async fn parse_all_banks<T>(
    bank_paths: impl IntoIterator<Item = (usize, String)>,
    archive_path: Arc<Path>,
    banks_parsed: Arc<AtomicUsize>,
    num_banks: usize,
    progress_tx: mpsc::Sender<f64>,
) -> Result<Vec<T>>
where
    T: Clone + Send + Sync + 'static,
    Vec<T>: DeserializeOwned,
{
    let mut parse_tasks = JoinSet::new();
    for (index, path) in bank_paths {
        let archive_path = archive_path.clone();
        parse_tasks.spawn(async move {
            parse_bank::<T>(&archive_path, index)
                .await
                .with_context(|| format!("failed to parse `{path}` as tag bank"))
        });
    }

    let mut total_bank = Vec::<T>::new();
    while let Some(bank) = parse_tasks.join_next().await {
        let bank = bank.context("import task canceled")??;
        total_bank.extend_from_slice(&bank);

        let banks_parsed = banks_parsed.fetch_add(1, atomic::Ordering::SeqCst) + 1;
        let progress = (banks_parsed as f64) / (num_banks as f64);
        _ = progress_tx.try_send(0.5 * progress);
    }
    Ok(total_bank)
}

async fn parse_bank<T>(path: &Path, entry_index: usize) -> Result<Vec<T>>
where
    Vec<T>: DeserializeOwned,
{
    let mut archive = open_archive(path).await?;
    let mut entry = archive
        .reader_with_entry(entry_index)
        .await
        .context("failed to read entry")?;
    let mut bank = Vec::new();
    entry
        .read_to_end_checked(&mut bank)
        .await
        .context("failed to read into memory")?;
    let bank = serde_json::from_slice::<Vec<T>>(&bank).context("failed to parse bank")?;
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
    source: DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    records: &mut Insert<Record>,
    frequencies: &mut Insert<FrequencyValue>,
    term_data: schema::Term,
    all_tags: &[GlossaryTag],
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

    records
        .insert(tx, source, term.clone(), &record)
        .await
        .context("failed to insert record")?;
    frequencies
        .insert(
            tx,
            source,
            term,
            FrequencyValue::Occurrence(term_data.score),
        )
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
    source: DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    records: &mut Insert<Record>,
    frequencies: &mut Insert<FrequencyValue>,
    index: &schema::Index,
    term_meta: schema::TermMeta,
) -> Result<()> {
    let headword = NormString::new(term_meta.expression);
    match term_meta.data {
        schema::TermMetaData::Frequency(frequency) => {
            // Yomitan dictionaries like VN Freq v2 seem to default to rank-based
            let frequency_mode = index.frequency_mode.unwrap_or(FrequencyMode::RankBased);
            let (record, reading) = to_frequency_and_reading(frequency_mode, frequency);
            let term = Term::new(headword, reading)
                .context("frequency term has no headword or reading")?;

            records
                .insert(tx, source, term.clone(), &record)
                .await
                .context("failed to insert frequency record")?;

            if let Some(value) = record.value {
                frequencies
                    .insert(tx, source, term, value)
                    .await
                    .context("failed to insert frequency sorting record")?;
            }
        }
        schema::TermMetaData::Pitch(pitch) => {
            for (record, reading) in to_pitches_and_readings(pitch) {
                let term = Term::new(headword.clone(), reading)
                    .context("pitch term has no headword or reading")?;

                records
                    .insert(tx, source, term, &record)
                    .await
                    .context("failed to insert pitch record")?;
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
