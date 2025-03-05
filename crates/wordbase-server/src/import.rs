use std::sync::Arc;

use anyhow::{Context as _, Result};
use futures::{StreamExt, TryStreamExt};
use sqlx::{Pool, Sqlite, Transaction};
use tokio::sync::Mutex;
use wordbase::{
    schema::{DictionaryId, ExpressionInfo, Frequency, Glossary, LookupInfo, Pitch},
    yomitan,
};

pub async fn term_bank(
    DictionaryId(source): DictionaryId,
    tx: Arc<Mutex<Transaction<'_, Sqlite>>>,
    bank: yomitan::TermBank,
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    for term in bank {
        let expression = term.expression.clone();
        let reading = term.reading.clone();
        async {
            sqlx::query!(
                "INSERT OR IGNORE INTO readings (source, expression, reading)
                VALUES ($1, $2, $3)",
                source,
                expression,
                reading,
            )
            .execute(&mut **tx.lock().await)
            .await
            .context("failed to insert into `readings`")?;

            for glossary in convert_term(term) {
                scratch.clear();
                postcard::to_io(&glossary, &mut scratch).context("failed to serialize data")?;
                let data = &scratch[..];

                sqlx::query!(
                    "INSERT INTO glossaries (source, expression, reading, data)
                    VALUES ($1, $2, $3, $4)",
                    source,
                    expression,
                    reading,
                    data
                )
                .execute(&mut **tx.lock().await)
                .await
                .context("failed to insert into `glossaries`")?;
            }

            anyhow::Ok(())
        }
        .await
        .with_context(|| format!("failed to import term {expression:?} ({reading:?})"))?;
    }
    Ok(())
}

fn convert_term(raw: yomitan::Term) -> impl Iterator<Item = Glossary> {
    raw.glossary
        .into_iter()
        .flat_map(|glossary| match glossary {
            yomitan::Glossary::Deinflection(_) => None,
            yomitan::Glossary::String(text)
            | yomitan::Glossary::Content(yomitan::GlossaryContent::Text { text }) => {
                Some(Glossary::Definition { text })
            }
            yomitan::Glossary::Content(yomitan::GlossaryContent::Image(image)) => {
                None // TODO
            }
            yomitan::Glossary::Content(yomitan::GlossaryContent::StructuredContent { content }) => {
                None // TODO
            }
        })
}

pub async fn term_meta_bank(
    DictionaryId(source): DictionaryId,
    tx: Arc<Mutex<Transaction<'_, Sqlite>>>,
    bank: yomitan::TermMetaBank,
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    for term_meta in bank {
        let expression = term_meta.expression.clone();
        async {
            match term_meta.data {
                yomitan::TermMetaData::Frequency(frequency) => {
                    for (reading, frequency) in convert_frequency(frequency) {
                        let reading = reading.unwrap_or_else(|| expression.clone());

                        scratch.clear();
                        postcard::to_io(&frequency, &mut scratch).context("failed to serialize data")?;
                        let data = &scratch[..];

                        sqlx::query!(
                            "INSERT INTO frequencies (source, expression, reading, data)
                            VALUES ($1, $2, $3, $4)",
                            source,
                            expression,
                            reading,
                            data,
                        )
                        .execute(&mut **tx.lock().await)
                        .await
                        .context("failed to insert into `frequencies`")?;
                    }
                }
                yomitan::TermMetaData::Pitch(pitch) => {
                    for (reading, pitch) in convert_pitch(pitch) {
                        scratch.clear();
                        postcard::to_io(&pitch, &mut scratch).context("failed to serialize data")?;
                        let data = &scratch[..];

                        sqlx::query!(
                            "INSERT INTO pitches (source, expression, reading, data) VALUES ($1, $2, $3, $4)",
                            source,
                            expression,
                            reading,
                            data,
                        )
                        .execute(&mut **tx.lock().await)
                        .await
                        .context("failed to insert into `pitches`")?;
                    }
                }
                yomitan::TermMetaData::Phonetic(_) => {}
            }

            anyhow::Ok(())
        }
        .await
        .with_context(|| format!("failed to import term meta {expression:?}"))?;
    }
    Ok(())
}

fn convert_frequency(
    raw: yomitan::TermMetaFrequency,
) -> impl Iterator<Item = (Option<String>, Frequency)> {
    let (reading, generic) = match raw {
        yomitan::TermMetaFrequency::Generic(generic) => (None, generic),
        yomitan::TermMetaFrequency::WithReading { reading, frequency } => {
            (Some(reading), frequency)
        }
    };

    let new = match generic {
        yomitan::GenericFrequencyData::Number(value) => Some(Frequency {
            value,
            display_value: None,
        }),
        yomitan::GenericFrequencyData::String(_) => None,
        yomitan::GenericFrequencyData::Complex {
            value,
            display_value,
        } => Some(Frequency {
            value,
            display_value,
        }),
    };

    new.map(|new| (reading, new)).into_iter()
}

fn convert_pitch(raw: yomitan::TermMetaPitch) -> impl Iterator<Item = (String, Pitch)> {
    raw.pitches.into_iter().map(move |pitch| {
        (
            raw.reading.clone(),
            Pitch {
                position: pitch.position,
                nasal: convert_pitch_position(pitch.nasal),
                devoice: convert_pitch_position(pitch.devoice),
            },
        )
    })
}

fn convert_pitch_position(raw: Option<yomitan::PitchPosition>) -> Vec<u64> {
    match raw {
        None => vec![],
        Some(yomitan::PitchPosition::One(position)) => vec![position],
        Some(yomitan::PitchPosition::Many(positions)) => positions,
    }
}

pub async fn lookup(db: &Pool<Sqlite>, lemma: String) -> Result<LookupInfo> {
    let (terms, frequencies, pitches, glossaries) = tokio::join!(
        fetch_terms(db, &lemma),
        fetch_frequencies(db, &lemma),
        fetch_pitches(db, &lemma),
        fetch_glossaries(db, &lemma),
    );
    let (terms, frequencies, pitches, glossaries) = (
        terms.context("failed to fetch terms")?,
        frequencies.context("failed to fetch frequencies")?,
        pitches.context("failed to fetch pitches")?,
        glossaries.context("failed to fetch glossaries")?,
    );

    Ok(LookupInfo {
        lemma,
        terms,
        frequencies,
        pitches,
        glossaries,
    })
}

async fn fetch_terms(db: &Pool<Sqlite>, lemma: &str) -> Result<Vec<ExpressionInfo>> {
    sqlx::query!(
        "SELECT source, expression, reading
        FROM readings
        WHERE expression = $1 OR reading = $1",
        lemma
    )
    .fetch(db)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        anyhow::Ok(ExpressionInfo {
            source: DictionaryId(record.source),
            expression: record.expression,
            reading: record.reading,
        })
    })
    .try_collect()
    .await
}

async fn fetch_frequencies(
    db: &Pool<Sqlite>,
    lemma: &str,
) -> Result<Vec<(ExpressionInfo, Frequency)>> {
    sqlx::query!(
        "SELECT source, expression, reading, data
        FROM frequencies
        WHERE expression = $1 OR reading = $1",
        lemma
    )
    .fetch(db)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        anyhow::Ok((
            ExpressionInfo {
                source: DictionaryId(record.source),
                expression: record.expression,
                reading: record.reading,
            },
            postcard::from_bytes(&record.data).context("failed to deserialize data")?,
        ))
    })
    .try_collect()
    .await
}

async fn fetch_pitches(db: &Pool<Sqlite>, lemma: &str) -> Result<Vec<(ExpressionInfo, Pitch)>> {
    sqlx::query!(
        "SELECT source, expression, reading, data
        FROM pitches
        WHERE expression = $1 OR reading = $1",
        lemma
    )
    .fetch(db)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        anyhow::Ok((
            ExpressionInfo {
                source: DictionaryId(record.source),
                expression: record.expression,
                reading: record.reading,
            },
            postcard::from_bytes(&record.data).context("failed to deserialize data")?,
        ))
    })
    .try_collect()
    .await
}

async fn fetch_glossaries(
    db: &Pool<Sqlite>,
    lemma: &str,
) -> Result<Vec<(ExpressionInfo, Glossary)>> {
    sqlx::query!(
        "SELECT source, expression, reading, data
        FROM glossaries
        WHERE expression = $1 OR reading = $1",
        lemma
    )
    .fetch(db)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        anyhow::Ok((
            ExpressionInfo {
                source: DictionaryId(record.source),
                expression: record.expression,
                reading: record.reading,
            },
            postcard::from_bytes(&record.data).context("failed to deserialize data")?,
        ))
    })
    .try_collect()
    .await
}

#[test]
fn test() {
    let x = Glossary::Definition { text: "hi".into() };
    let b = postcard::to_stdvec(&x).unwrap();
    postcard::from_bytes::<Glossary>(&b).unwrap();
}
