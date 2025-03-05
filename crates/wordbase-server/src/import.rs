use std::{
    convert::Infallible,
    io::Cursor,
    path::Path,
    sync::atomic::{self, AtomicUsize},
};

use anyhow::{Context as _, Result};
use sqlx::{Pool, Sqlite, Transaction};
use tokio::{fs, sync::Mutex};
use tracing::info;
use wordbase::{
    schema::{DictionaryId, Frequency, Glossary, Pitch},
    yomitan,
};

use crate::db::data_kind;

pub async fn from_yomitan(db: Pool<Sqlite>, path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let archive = fs::read(path)
        .await
        .context("failed to read file into memory")?;

    let (parser, index) = yomitan::Parse::new(|| Ok::<_, Infallible>(Cursor::new(&archive)))
        .context("failed to parse")?;
    let term_banks_left = AtomicUsize::new(parser.term_banks().len());
    let term_meta_banks_left = AtomicUsize::new(parser.term_meta_banks().len());
    info!("{}", index.title);
    info!(
        "    term banks: {} | term meta banks: {}",
        term_banks_left.load(atomic::Ordering::Relaxed),
        term_meta_banks_left.load(atomic::Ordering::Relaxed)
    );

    let term_bank = Mutex::new(yomitan::TermBank::default());
    let term_meta_bank = Mutex::new(yomitan::TermMetaBank::default());

    info!("Parsing...");
    let span = tracing::Span::current();
    parser
        .run(
            |_, _| {},
            |_, bank| {
                let _span = span.enter();
                term_bank.blocking_lock().extend_from_slice(&bank);

                let left = term_banks_left.fetch_sub(1, atomic::Ordering::SeqCst) - 1;
                if left % 10 == 0 {
                    info!("{left} term banks left");
                }
            },
            |_, bank| {
                let _span = span.enter();
                term_meta_bank.blocking_lock().extend_from_slice(&bank);

                let left = term_meta_banks_left.fetch_sub(1, atomic::Ordering::SeqCst) - 1;
                if left % 10 == 0 {
                    info!("{left} term meta banks left");
                }
            },
            |_, _| {},
            |_, _| {},
        )
        .context("failed to parse banks")?;
    info!("Parse complete, starting transaction...");
    let mut tx = db.begin().await.context("failed to begin transaction")?;

    let dictionary_id = {
        info!("Writing dictionary record...");
        let result = sqlx::query!(
            "INSERT INTO dictionaries (title, revision)
            VALUES ($1, $2)",
            index.title,
            index.revision
        )
        .execute(&mut *tx)
        .await
        .context("failed to insert dictionary")?;

        DictionaryId(result.last_insert_rowid())
    };

    info!("Importing term bank");
    from_term_bank(dictionary_id, &mut tx, term_bank.into_inner())
        .await
        .context("failed to import term bank")?;
    info!("Importing term meta bank");
    from_term_meta_bank(dictionary_id, &mut tx, term_meta_bank.into_inner())
        .await
        .context("failed to import term meta bank")?;

    info!("Committing...");
    tx.commit().await.context("failed to commit transaction")?;

    info!("*=* COMPLETE *=*");
    Ok(())
}

fn none_if_empty(s: String) -> Option<String> {
    if s.trim().is_empty() { None } else { Some(s) }
}

async fn from_term_bank(
    DictionaryId(source): DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    bank: yomitan::TermBank,
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    let mut records_left = bank.len();
    for term in bank {
        let expression = term.expression.clone();
        let reading = none_if_empty(term.reading.clone());

        async {
            for glossary in to_glossaries(term) {
                scratch.clear();
                postcard::to_io(&glossary, &mut scratch).context("failed to serialize data")?;
                let data = &scratch[..];

                sqlx::query!(
                    "INSERT INTO terms (source, expression, reading, data_kind, data)
                    VALUES ($1, $2, $3, $4, $5)",
                    source,
                    expression,
                    reading,
                    data_kind::GLOSSARY,
                    data
                )
                .execute(&mut **tx)
                .await
                .context("failed to insert record")?;
            }

            records_left -= 1;
            if records_left % 10000 == 0 {
                info!("IMPORT: {records_left} term records left");
            }

            anyhow::Ok(())
        }
        .await
        .with_context(|| format!("failed to import term {expression:?} ({reading:?})"))?;
    }
    Ok(())
}

async fn from_term_meta_bank(
    DictionaryId(source): DictionaryId,
    tx: &mut Transaction<'_, Sqlite>,
    bank: yomitan::TermMetaBank,
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    let mut records_left = bank.len();
    for term_meta in bank {
        let expression = term_meta.expression.clone();
        async {
            match term_meta.data {
                yomitan::TermMetaData::Frequency(frequency) => {
                    for (reading, frequency) in to_frequencies(frequency) {
                        scratch.clear();
                        postcard::to_io(&frequency, &mut scratch)
                            .context("failed to serialize data")?;
                        let data = &scratch[..];

                        sqlx::query!(
                            "INSERT INTO terms (source, expression, reading, data_kind, data)
                            VALUES ($1, $2, $3, $4, $5)",
                            source,
                            expression,
                            reading,
                            data_kind::FREQUENCY,
                            data,
                        )
                        .execute(&mut **tx)
                        .await
                        .context("failed to insert record")?;
                    }
                }
                yomitan::TermMetaData::Pitch(pitch) => {
                    for (reading, pitch) in to_pitch(pitch) {
                        scratch.clear();
                        postcard::to_io(&pitch, &mut scratch)
                            .context("failed to serialize data")?;
                        let data = &scratch[..];

                        sqlx::query!(
                            "INSERT INTO terms (source, expression, reading, data_kind, data)
                            VALUES ($1, $2, $3, $4, $5)",
                            source,
                            expression,
                            reading,
                            data_kind::PITCH,
                            data,
                        )
                        .execute(&mut **tx)
                        .await
                        .context("failed to insert record")?;
                    }
                }
                yomitan::TermMetaData::Phonetic(_) => {}
            }

            records_left -= 1;
            if records_left % 10000 == 0 {
                info!("IMPORT: {records_left} term meta records left");
            }

            anyhow::Ok(())
        }
        .await
        .with_context(|| format!("failed to import term meta {expression:?}"))?;
    }
    Ok(())
}

fn to_glossaries(raw: yomitan::Term) -> impl Iterator<Item = Glossary> {
    raw.glossary
        .into_iter()
        .flat_map(|glossary| match glossary {
            yomitan::Glossary::Deinflection(_) => None,
            yomitan::Glossary::String(text)
            | yomitan::Glossary::Content(yomitan::GlossaryContent::Text { text }) => {
                Some(Glossary { text })
            }
            yomitan::Glossary::Content(yomitan::GlossaryContent::Image(_image)) => {
                None // TODO
            }
            yomitan::Glossary::Content(yomitan::GlossaryContent::StructuredContent {
                content: _,
            }) => {
                None // TODO
            }
        })
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
        yomitan::GenericFrequencyData::Number(value) => Some(Frequency {
            rank: value,
            display_rank: None,
        }),
        yomitan::GenericFrequencyData::String(_) => None,
        yomitan::GenericFrequencyData::Complex {
            value,
            display_value,
        } => Some(Frequency {
            rank: value,
            display_rank: display_value,
        }),
    };

    frequency.map(|new| (reading, new)).into_iter()
}

fn to_pitch(raw: yomitan::TermMetaPitch) -> impl Iterator<Item = (String, Pitch)> {
    raw.pitches.into_iter().map(move |pitch| {
        (
            raw.reading.clone(),
            Pitch {
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
