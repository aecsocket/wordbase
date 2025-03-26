mod parse;
mod schema;

use {
    super::{ImportError, ImportFormat, ImportTracker, insert_term},
    crate::{
        CHANNEL_BUF_CAP, Engine,
        import::{dictionary_exists_by_name, insert_dictionary},
    },
    anyhow::{Context as _, Result},
    bytes::Bytes,
    futures::future::BoxFuture,
    parse::{Parse, ParseError},
    sqlx::{Sqlite, Transaction},
    std::{
        convert::Infallible,
        io::{Cursor, Read, Seek},
        iter,
        sync::atomic::{self, AtomicUsize},
    },
    tokio::sync::{Mutex, mpsc, oneshot},
    tracing::debug,
    wordbase::{
        DictionaryFormat, DictionaryId, DictionaryMeta, Term,
        format::{
            self,
            yomitan::{GlossaryTag, structured},
        },
        lang,
        record::Frequency,
    },
    zip::ZipArchive,
};

pub struct Yomitan;

impl ImportFormat for Yomitan {
    fn validate(&self, archive: Bytes) -> BoxFuture<Result<()>> {
        Box::pin(async move {
            blocking::unblock(|| {
                let data = Cursor::new(archive);
                let mut archive = ZipArchive::new(data).context("not a zip archive")?;
                archive
                    .by_name(INDEX_PATH)
                    .with_context(|| format!("no `{INDEX_PATH}` in archive"))?;
                Ok(())
            })
            .await
        })
    }

    fn import(
        &self,
        engine: &Engine,
        archive: Bytes,
        send_tracker: oneshot::Sender<ImportTracker>,
    ) -> BoxFuture<Result<(), ImportError>> {
        todo!()
    }
}

const INDEX_PATH: &str = "index.json";

impl Engine {
    pub(super) async fn import_dictionary_yomitan<R: Read + Seek, E: Send>(
        &self,
        new_reader: impl Fn() -> Result<R, E> + Send + Sync,
        send_tracker: oneshot::Sender<ImportTracker>,
    ) -> Result<(), ImportError>
    where
        ParseError<E>: std::error::Error + Send + Sync + 'static,
    {
        let (parser, index) = Parse::new(new_reader).context("failed to parse index")?;
        let meta = DictionaryMeta {
            format: DictionaryFormat::Yomitan,
            name: index.title,
            version: index.revision,
            description: index.description,
            url: index.url,
        };
        let banks_len = parser.tag_banks().len()
            + parser.term_banks().len()
            + parser.term_meta_banks().len()
            + parser.kanji_banks().len()
            + parser.kanji_meta_banks().len();
        let banks_done = AtomicUsize::new(0);

        debug!(
            "{:?} version {:?} - {banks_len} items",
            meta.name, meta.version
        );
        let (send_progress, recv_progress) = mpsc::channel(CHANNEL_BUF_CAP);
        _ = send_tracker.send(ImportTracker {
            meta: meta.clone(),
            recv_progress,
        });

        let already_exists = dictionary_exists_by_name(&self.db, &meta.name)
            .await
            .context("failed to check if dictionary exists")?;
        if already_exists {
            debug!("Dictionary already exists");
            return Err(ImportError::AlreadyExists);
        }

        let tag_bank = Mutex::new(schema::TagBank::default());
        let term_bank = Mutex::new(schema::TermBank::default());
        let term_meta_bank = Mutex::new(schema::TermMetaBank::default());
        let kanji_bank = Mutex::new(schema::KanjiBank::default());
        let kanji_meta_bank = Mutex::new(schema::KanjiMetaBank::default());

        let notify_parsed = || {
            let banks_done = banks_done.fetch_add(1, atomic::Ordering::SeqCst) + 1;
            let frac_done = 0.5 * ((banks_done as f64) / (banks_len as f64));
            _ = send_progress.try_send(frac_done);
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
        let tag_bank = tag_bank.into_inner();
        let term_bank = term_bank.into_inner();
        let term_meta_bank = term_meta_bank.into_inner();
        let kanji_bank = kanji_bank.into_inner();
        let kanji_meta_bank = kanji_meta_bank.into_inner();

        let records_len =
            term_bank.len() + term_meta_bank.len() + kanji_bank.len() + kanji_meta_bank.len();
        let records_done = AtomicUsize::new(0);
        if records_len == 0 {
            debug!("Parse complete, no records to insert");
            return Err(ImportError::NoRecords);
        }
        debug!("Parse complete, inserting {records_len} records");

        debug!("Waiting for insert lock");
        let _tx_lock = self.importer.insert_lock.lock().await;
        debug!("Lock acquired, starting transaction");
        let mut tx = self
            .db
            .begin()
            .await
            .context("failed to begin transaction")?;
        debug!("Started transaction");

        let dictionary_id = insert_dictionary(&mut tx, &meta)
            .await
            .context("failed to insert dictionary")?;
        debug!("Inserted with ID {dictionary_id:?}");

        let mut all_tags = tag_bank.into_iter().map(to_term_tag).collect::<Vec<_>>();
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
        drop(send_progress);

        tx.commit().await.context("failed to commit transaction")?;

        self.sync_dictionaries()
            .await
            .context("failed to sync dictionaries")?;
        Ok(())
    }
}

fn sanitize(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

async fn import_term(
    source: DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    term_data: schema::Term,
    all_tags: &[GlossaryTag],
    scratch: &mut Vec<u8>,
) -> Result<()> {
    let term = Term::from_pair(
        sanitize(term_data.reading.clone()),
        sanitize(term_data.expression.clone()),
    )
    .context("term has no headword or reading")?;

    let tags = match_tags(
        all_tags,
        term_data.definition_tags.as_deref().unwrap_or_default(),
    )
    .cloned()
    .collect::<Vec<_>>();
    let glossary = term_data
        .glossary
        .into_iter()
        .filter_map(to_content)
        .collect();
    let glossary = format::yomitan::Record {
        popularity: term_data.score,
        tags,
        glossary,
    };

    insert_term(tx, source, &term, &glossary, scratch).await?;
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
                let term = reading.map_or_else(
                    || Term::new(headword.clone()),
                    |reading| Term::with_reading(headword.clone(), reading),
                );

                insert_term(tx, source, &term, &record, scratch)
                    .await
                    .context("failed to insert frequency record")?;
            }
        }
        schema::TermMetaData::Pitch(pitch) => {
            for (reading, record) in to_pitch(pitch) {
                insert_term(
                    tx,
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

fn to_pitch(raw: schema::TermMetaPitch) -> impl Iterator<Item = (String, lang::jpn::Pitch)> {
    raw.pitches.into_iter().map(move |pitch| {
        (
            raw.reading.clone(),
            lang::jpn::Pitch {
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
