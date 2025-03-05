use std::sync::Arc;

use anyhow::{Context as _, Result};
use derive_more::From;
use serde::{Deserialize, Serialize};
use sqlx::{Sqlite, Transaction};
use tokio::sync::Mutex;
use wordbase::{
    dict::{DictionaryId, Frequency},
    yomitan,
};

pub async fn term_bank(
    dictionary_id: DictionaryId,
    tx: Arc<Mutex<Transaction<'_, Sqlite>>>,
    bank: yomitan::TermBank,
) -> Result<()> {
    for term in bank {
        let expression = term.expression.clone();
        let reading = term.reading.clone();
        let data = "{}";
        sqlx::query!(
            "INSERT INTO terms (dictionary, expression, reading, data) VALUES ($1, $2, $3, $4)",
            dictionary_id.0,
            expression,
            reading,
            data,
        )
        .execute(&mut **tx.lock().await)
        .await
        .with_context(|| format!("failed to insert term {expression:?} ({reading:?})"))?;
    }
    Ok(())
}

pub async fn term_meta_bank(
    dictionary_id: DictionaryId,
    tx: Arc<Mutex<Transaction<'_, Sqlite>>>,
    bank: yomitan::TermMetaBank,
) -> Result<()> {
    let mut serialized_data = Vec::<u8>::new();
    for term_meta in bank {
        let expression = term_meta.expression.clone();
        let expression2 = expression.clone();
        async {
            let Some(data) = convert_term_meta(term_meta) else {
                return anyhow::Ok(());
            };
            postcard::to_io(&data, &mut serialized_data).context("failed to serialize data")?;
            let data = &serialized_data[..];

            sqlx::query!(
                "INSERT INTO term_meta (dictionary, expression, data) VALUES ($1, $2, $3)",
                dictionary_id.0,
                expression2,
                data,
            )
            .execute(&mut **tx.lock().await)
            .await
            .with_context(|| format!("failed to insert"))?;

            Ok(())
        }
        .await
        .with_context(|| format!("failed to import term meta {expression:?}"))?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, From)]
pub enum TermMeta {
    Frequency(Frequency),
    Pitch {
        reading: String,
        variants: Vec<PitchVariant>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchVariant {
    pub position: u64,
    pub nasal: Vec<u64>,
    pub devoice: Vec<u64>,
}

fn convert_term_meta(term_meta: yomitan::TermMeta) -> Option<TermMeta> {
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
}

fn convert_pitch_position(position: Option<yomitan::PitchPosition>) -> Vec<u64> {
    match position {
        None => vec![],
        Some(yomitan::PitchPosition::One(position)) => vec![position],
        Some(yomitan::PitchPosition::Many(positions)) => positions,
    }
}
