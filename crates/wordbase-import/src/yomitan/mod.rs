use {
    eyre::{Context as _, Result, eyre},
    rayon::prelude::*,
    serde::de::DeserializeOwned,
    std::sync::atomic::{self, AtomicUsize},
    tracing::{debug, trace, trace_span},
    wordbase_api::{
        DictionaryMeta, FrequencyValue, Term,
        v1::{
            jpn::PitchPosition,
            yomitan::{
                Frequency, Glossary, GlossaryTag, Kanji, PhoneticTranscription, Phonetics, Pitch,
                structured,
            },
        },
    },
    wordbase_storage::{
        archive::{Archive, OpenArchive},
        import::{
            FinishImport, ImportBatch, ImportBatchExt, ImportProgress, ImportTransaction,
            StartImport,
        },
    },
    zip::ZipArchive,
};

mod schema;

/// Importer for [`wordbase_api::v1::yomitan`].
pub struct Yomitan;

impl StartImport for Yomitan {
    fn start<'a>(
        &self,
        open_archive: &'a dyn OpenArchive,
    ) -> Result<(DictionaryMeta, Box<dyn FinishImport + 'a>)> {
        start(open_archive)
    }
}

fn start(open_archive: &dyn OpenArchive) -> Result<(DictionaryMeta, Box<dyn FinishImport + '_>)> {
    let mut archive = archive_reader(open_archive)?;

    let index = {
        let file = archive
            .by_name(schema::INDEX_PATH)
            .wrap_err_with(|| eyre!("missing `{}`", schema::INDEX_PATH))?;

        debug!("Reading index");
        serde_json::from_reader::<_, schema::Index>(file).wrap_err("failed to parse index")?
    };

    let mut meta = DictionaryMeta::new(&index.title);
    meta.version = Some(index.revision.clone());
    meta.description = index.description.clone();
    meta.url = index.url.clone();
    meta.attribution = index.attribution.clone();

    Ok((
        meta,
        Box::new(move |txn: &dyn ImportTransaction, tx_progress| {
            let index = &index;
            finish(open_archive, index, txn, &tx_progress)
        }),
    ))
}

fn archive_reader(open_archive: &dyn OpenArchive) -> Result<ZipArchive<impl Archive + 'static>> {
    let archive = open_archive
        .open_archive()
        .wrap_err("failed to open archive")?;
    ZipArchive::new(archive).wrap_err("failed to read zip archive")
}

fn finish(
    open_archive: &dyn OpenArchive,
    index: &schema::Index,
    txn: &dyn ImportTransaction,
    tx_progress: &async_channel::Sender<ImportProgress>,
) -> Result<()> {
    let archive = archive_reader(open_archive)?;

    let (mut tag_banks, mut term_banks, mut term_meta_banks, mut kanji_banks, mut kanji_meta_banks) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for path in archive.file_names() {
        trace!("Processing {path:?}");

        if schema::TAG_BANK_PATTERN.is_match(path) {
            tag_banks.push(path.to_owned());
        } else if schema::TERM_BANK_PATTERN.is_match(path) {
            term_banks.push(path.to_owned());
        } else if schema::TERM_META_BANK_PATTERN.is_match(path) {
            term_meta_banks.push(path.to_owned());
        } else if schema::KANJI_BANK_PATTERN.is_match(path) {
            kanji_banks.push(path.to_owned());
        } else if schema::KANJI_META_BANK_PATTERN.is_match(path) {
            kanji_meta_banks.push(path.to_owned());
        }
    }
    let num_banks =
        term_banks.len() + term_meta_banks.len() + kanji_banks.len() + kanji_meta_banks.len();
    debug!(
        "Found {num_banks} banks ({} term + {} term meta + {} kanji + {} kanji meta)",
        term_banks.len(),
        term_meta_banks.len(),
        kanji_banks.len(),
        kanji_meta_banks.len()
    );

    debug!("Processing banks");
    let banks_done = AtomicUsize::new(0);
    let bank_cx = BankContext {
        open_archive,
        txn,
        num_banks,
        banks_done: &banks_done,
        index,
        tx_progress,
    };

    let do_term_banks = || {
        let mut tags = tag_banks
            .into_iter()
            .try_fold(Vec::new(), |mut acc, path| {
                let bank = parse_bank::<schema::Tag>(&bank_cx, &path)
                    .wrap_err_with(|| eyre!("failed to parse term bank `{path}`"))?;
                let tags = bank.into_iter().map(|tag| GlossaryTag {
                    name: tag.name,
                    category: tag.category,
                    description: tag.notes,
                    order: tag.order,
                });
                acc.extend(tags);
                eyre::Ok(acc)
            })?;
        tags.sort_unstable_by_key(|tag| tag.order);

        term_banks.into_par_iter().try_for_each(|path| {
            import_bank(&bank_cx, &path, |tables, data, _| {
                import_term(tables, data, &tags)
            })
            .wrap_err_with(|| eyre!("failed to import term bank `{path}`"))
        })
    };
    let do_term_meta_banks = || {
        term_meta_banks.into_par_iter().try_for_each(|path| {
            import_bank(&bank_cx, &path, import_term_meta)
                .wrap_err_with(|| eyre!("failed to import term meta bank `{path}`"))
        })
    };
    let do_kanji_banks = || {
        kanji_banks.into_par_iter().try_for_each(|path| {
            import_bank(&bank_cx, &path, import_kanji)
                .wrap_err_with(|| eyre!("failed to import kanji bank `{path}`"))
        })
    };
    let do_kanji_meta_banks = || {
        kanji_meta_banks.into_par_iter().try_for_each(|path| {
            import_bank(&bank_cx, &path, import_kanji_meta)
                .wrap_err_with(|| eyre!("failed to import kanji meta bank `{path}`"))
        })
    };

    let ((r1, r2), (r3, r4)) = rayon::join(
        || rayon::join(do_term_banks, do_term_meta_banks),
        || rayon::join(do_kanji_banks, do_kanji_meta_banks),
    );
    _ = (r1?, r2?, r3?, r4?);
    Ok(())
}

struct BankContext<'cx> {
    open_archive: &'cx dyn OpenArchive,
    txn: &'cx dyn ImportTransaction,
    num_banks: usize,
    banks_done: &'cx AtomicUsize,
    index: &'cx schema::Index,
    tx_progress: &'cx async_channel::Sender<ImportProgress>,
}

fn parse_bank<E>(cx: &BankContext, path: &str) -> Result<Vec<E>>
where
    Vec<E>: DeserializeOwned,
{
    let mut archive = archive_reader(cx.open_archive)?;
    let file = archive.by_name(path).wrap_err("file does not exist")?;
    serde_json::from_reader::<_, Vec<E>>(file).wrap_err("failed to parse file")
}

fn import_bank<E>(
    cx: &BankContext,
    path: &str,
    mut import_item: impl FnMut(&mut dyn ImportBatch, E, &schema::Index) -> Result<()>,
) -> Result<()>
where
    Vec<E>: DeserializeOwned,
{
    let _span = trace_span!("bank", ?path).entered();
    let bank = parse_bank(cx, path)?;

    {
        let mut batch = cx.txn.batch().wrap_err("failed to begin writing batch")?;
        for data in bank {
            import_item(&mut *batch, data, cx.index)?;
        }
    }

    let banks_done = cx.banks_done.fetch_add(1, atomic::Ordering::SeqCst) + 1;
    debug!("{banks_done}/{} banks imported", cx.num_banks);

    let progress = banks_done as f64 / cx.num_banks as f64;
    let _ = cx.tx_progress.try_send(ImportProgress { progress });

    Ok(())
}

fn import_term(
    batch: &mut dyn ImportBatch,
    data: schema::Term,
    all_tags: &[GlossaryTag],
) -> Result<()> {
    let to_content = |raw: schema::Glossary| match raw {
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
    };

    let term = Term::from_full(data.expression, data.reading)?;
    let record = Glossary {
        popularity: data.score,
        tags: data
            .definition_tags
            .as_deref()
            .unwrap_or("")
            // spaces in tags like "rarely used form" aren't actually spaces:
            // `rarely\u{a0}used\u{a0}form`
            // so we split on ` `, not `\u{a0}`
            .split(' ')
            .filter_map(|name| all_tags.iter().find(|tag| tag.name == name))
            .cloned()
            .collect(),
        content: data.glossary.into_iter().filter_map(to_content).collect(),
    };

    (|| {
        let record_id = batch.insert_record(record)?;
        batch.insert_term(&term, record_id)?;
        eyre::Ok(())
    })()
    .wrap_err_with(|| eyre!("failed to insert glossary for {term}"))
}

fn import_term_meta(
    batch: &mut dyn ImportBatch,
    data: schema::TermMeta,
    index: &schema::Index,
) -> Result<()> {
    match data.data {
        schema::TermMetaData::Frequency(frequency) => {
            let (term, record) = match frequency {
                schema::TermMetaFrequency::Generic(frequency) => (
                    Term::from_headword(data.expression)?,
                    map_generic_frequency_data(index, frequency),
                ),
                schema::TermMetaFrequency::WithReading { reading, frequency } => (
                    Term::from_full(data.expression, reading)?,
                    map_generic_frequency_data(index, frequency),
                ),
            };

            (|| {
                let record_id = batch.insert_record(record)?;
                batch.insert_term(&term, record_id)?;
                eyre::Ok(())
            })()
            .wrap_err_with(|| eyre!("failed to insert frequency for {term}"))?;
        }
        schema::TermMetaData::Pitch(pitch) => {
            let map_positions = |raw: Option<schema::PitchPosition>| match raw {
                None => vec![],
                Some(schema::PitchPosition::One(pos)) => vec![PitchPosition(pos)],
                Some(schema::PitchPosition::Many(pos)) => {
                    pos.into_iter().map(PitchPosition).collect()
                }
            };

            let term = Term::from_full(data.expression, pitch.reading)?;

            (|| {
                for pitch in pitch.pitches {
                    let record = Pitch {
                        position: PitchPosition(pitch.position),
                        nasal: map_positions(pitch.nasal),
                        devoice: map_positions(pitch.devoice),
                    };

                    let record_id = batch.insert_record(record)?;
                    batch.insert_term(&term, record_id)?;
                }
                eyre::Ok(())
            })()
            .wrap_err_with(|| eyre!("failed to insert pitch for {term}"))?;
        }
        schema::TermMetaData::Phonetic(phonetic) => {
            let term = Term::from_full(data.expression, phonetic.reading)?;
            let record = Phonetics {
                transcriptions: phonetic
                    .transcriptions
                    .into_iter()
                    .map(|ts| PhoneticTranscription {
                        ipa: ts.ipa,
                        tags: ts.tags,
                    })
                    .collect(),
            };

            (|| {
                let record_id = batch.insert_record(record)?;
                batch.insert_term(&term, record_id)?;
                eyre::Ok(())
            })()
            .wrap_err_with(|| eyre!("failed to insert phonetics for {term}"))?;
        }
    }
    Ok(())
}

fn import_kanji(
    batch: &mut dyn ImportBatch,
    data: schema::Kanji,
    _index: &schema::Index,
) -> Result<()> {
    let term = Term::from_headword(data.character)?;
    let record = Kanji {
        onyomi: data
            .onyomi
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
        kunyomi: data
            .kunyomi
            .split_whitespace()
            .map(ToOwned::to_owned)
            .collect(),
        meanings: data.meanings,
        extra: data.stats.into_iter().collect(),
    };

    (|| {
        let record_id = batch.insert_record(record)?;
        batch.insert_term(&term, record_id)?;
        eyre::Ok(())
    })()
    .wrap_err_with(|| eyre!("failed to insert kanji for {term}"))
}

fn import_kanji_meta(
    batch: &mut dyn ImportBatch,
    data: schema::KanjiMeta,
    index: &schema::Index,
) -> Result<()> {
    let term = Term::from_headword(data.character)?;
    let record = map_generic_frequency_data(index, data.data);

    (|| {
        let record_id = batch.insert_record(record)?;
        batch.insert_term(&term, record_id)?;
        eyre::Ok(())
    })()
    .wrap_err_with(|| eyre!("failed to insert kanji frequency for {term}"))
}

fn map_generic_frequency_data(
    index: &schema::Index,
    data: schema::GenericFrequencyData,
) -> Frequency {
    // dictionaries like VN Freq v2 seem to default to rank-based
    let mode = index
        .frequency_mode
        .unwrap_or(schema::FrequencyMode::RankBased);

    let map_value = |n: i64| match mode {
        schema::FrequencyMode::OccurrenceBased => FrequencyValue::Occurrence(n),
        schema::FrequencyMode::RankBased => FrequencyValue::Rank(n),
    };

    match data {
        schema::GenericFrequencyData::String(s) => Frequency {
            value: s.parse().map(map_value).ok(),
            display: Some(s),
        },
        schema::GenericFrequencyData::Number(n) => Frequency {
            value: Some(map_value(n)),
            display: None,
        },
        schema::GenericFrequencyData::Complex {
            value,
            display_value,
        } => Frequency {
            value: Some(map_value(value)),
            display: display_value,
        },
    }
}
