use std::sync::Arc;

use anyhow::{Context as _, Result};
use sqlx::{Sqlite, Transaction};
use tokio::sync::Mutex;
use wordbase::{
    dict::{DictionaryId, Frequency, PitchVariant},
    yomitan,
};

use crate::format::{Term, TermMeta};

pub async fn term_bank(
    dictionary_id: DictionaryId,
    tx: Arc<Mutex<Transaction<'_, Sqlite>>>,
    bank: yomitan::TermBank,
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    for term in bank {
        let expression = term.expression.clone();
        let reading = term.reading.clone();
        for data in convert_term(term) {
            async {
                scratch.clear();
                postcard::to_io(&data, &mut scratch).context("failed to serialize data")?;
                let data = &scratch[..];
    
                sqlx::query!(
                    "INSERT INTO terms (dictionary, expression, reading, data) VALUES ($1, $2, $3, $4)",
                    dictionary_id.0,
                    expression,
                    reading,
                    data,
                )
                .execute(&mut **tx.lock().await)
                .await?;
                anyhow::Ok(())
            }
            .await
            .with_context(|| format!("failed to import term {expression:?} ({reading:?})"))?;
        }
    }
    Ok(())
}

fn convert_term(term: yomitan::Term) -> impl Iterator<Item = Term> {
    term.glossary.into_iter().flat_map(|glossary| match glossary {
        yomitan::Glossary::Deinflection(_) => None,
        yomitan::Glossary::String(definition) | yomitan::Glossary::Content(yomitan::GlossaryContent::Text { text: definition }) => {
            Some(Term::Definition(definition))
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
    dictionary_id: DictionaryId,
    tx: Arc<Mutex<Transaction<'_, Sqlite>>>,
    bank: yomitan::TermMetaBank,
) -> Result<()> {
    let mut scratch = Vec::<u8>::new();
    for term_meta in bank {
        let expression = term_meta.expression.clone();
        for data in convert_term_meta(term_meta) {
            async {
                scratch.clear();
                postcard::to_io(&data, &mut scratch).context("failed to serialize data")?;
                let data = &scratch[..];

                sqlx::query!(
                    "INSERT INTO term_meta (dictionary, expression, data) VALUES ($1, $2, $3)",
                    dictionary_id.0,
                    expression,
                    data,
                )
                .execute(&mut **tx.lock().await)
                .await?;
                anyhow::Ok(())
            }
            .await
            .with_context(|| format!("failed to import term meta {expression:?}"))?;
        }
    }
    Ok(())
}

fn convert_term_meta(term_meta: yomitan::TermMeta) -> impl Iterator<Item = TermMeta> {
    match term_meta.data {
        yomitan::TermMetaData::Frequency(frequency) => {
            let (yomitan::TermMetaFrequency::Generic(frequency)
            | yomitan::TermMetaFrequency::WithReading {
                reading: _,
                frequency,
            }) = frequency;

            match frequency {
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
            }
            .map(TermMeta::from)
        }
        yomitan::TermMetaData::Pitch(pitch) => Some(TermMeta::Pitch {
            reading: pitch.reading,
            variants: pitch
                .pitches
                .into_iter()
                .map(|pitch| PitchVariant {
                    position: pitch.position,
                    nasal: convert_pitch_position(pitch.nasal),
                    devoice: convert_pitch_position(pitch.devoice),
                })
                .collect(),
        }),
        yomitan::TermMetaData::Phonetic(_) => None,
    }
    .into_iter()
}

fn convert_pitch_position(position: Option<yomitan::PitchPosition>) -> Vec<u64> {
    match position {
        None => vec![],
        Some(yomitan::PitchPosition::One(position)) => vec![position],
        Some(yomitan::PitchPosition::Many(positions)) => positions,
    }
}
