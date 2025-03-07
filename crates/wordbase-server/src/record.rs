use std::io;

use anyhow::{Context as _, Result, bail};
use futures::{StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Sqlite};
use wordbase::{
    Dictionary, DictionaryId, Frequency, Glossary, Term, lang::jp, protocol::DictionaryNotFound,
};

// keep this up to date with `setup_db.sql`
pub mod data_kind {
    pub const GLOSSARY: u8 = 1;
    pub const FREQUENCY: u8 = 2;
    pub const JP_PITCH: u8 = 3;
}

pub fn serialize(
    value: &impl Serialize,
    writer: impl io::Write,
) -> Result<(), rmp_serde::encode::Error> {
    value.serialize(&mut rmp_serde::Serializer::new(writer))
}

pub fn deserialize<'a, T: Deserialize<'a>>(buf: &'a [u8]) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(buf)
}

// pub async fn lookup(db: &Pool<Sqlite>, lemma: String) -> Result<LookupInfo> {
//     let mut info = LookupInfo {
//         lemma: lemma.clone(),
//         ..Default::default()
//     };
//     let mut records = sqlx::query!(
//         "SELECT source, headword, reading, data_kind, data
//         FROM terms t
//         LEFT JOIN dictionaries
//             ON t.source = dictionaries.id
//         WHERE
//             dictionaries.enabled = TRUE
//             AND (headword = $1 OR reading = $1)",
//         lemma
//     )
//     .fetch(db);
//     while let Some(record) = records.next().await {
//         let record = record.context("failed to fetch record")?;
//         let source = DictionaryId(record.source);
//         let term = Term {
//             headword: record.headword,
//             reading: record.reading,
//         };

//         match u8::try_from(record.data_kind) {
//             Ok(data_kind::GLOSSARY) => {
//                 let data = deserialize::<Glossary>(&record.data)
//                     .context("failed to deserialize glossary data")?;
//                 info.glossaries.push((source, term, data));
//             }
//             Ok(data_kind::FREQUENCY) => {
//                 let data = deserialize::<Frequency>(&record.data)
//                     .context("failed to deserialize frequency data")?;
//                 info.frequencies.push((source, term, data));
//             }
//             Ok(data_kind::JP_PITCH) => {
//                 let data = deserialize::<jp::Pitch>(&record.data)
//                     .context("failed to deserialize jp_pitch data")?;
//                 info.pitches.push((source, term, data));
//             }
//             _ => bail!("invalid data kind {}", record.data_kind),
//         }
//     }

//     Ok(info)
// }

pub async fn list_dictionaries(db: &Pool<Sqlite>) -> Result<Vec<Dictionary>> {
    sqlx::query!(
        r#"SELECT id as "id!", name, version, position, enabled
        FROM dictionaries
        ORDER BY position"#
    )
    .fetch(db)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        anyhow::Ok(Dictionary {
            id: DictionaryId(record.id),
            name: record.name,
            version: record.version,
            position: record.position,
            enabled: record.enabled,
        })
    })
    .try_collect::<Vec<_>>()
    .await
}

pub async fn remove_dictionary(
    db: &Pool<Sqlite>,
    dictionary_id: DictionaryId,
) -> Result<Result<(), DictionaryNotFound>> {
    let result = sqlx::query!("DELETE FROM dictionaries WHERE id = $1", dictionary_id.0)
        .execute(db)
        .await
        .context("failed to delete record")?;
    Ok(if result.rows_affected() > 0 {
        Ok(())
    } else {
        Err(DictionaryNotFound)
    })
}

pub async fn set_dictionary_enabled(
    db: &Pool<Sqlite>,
    dictionary_id: DictionaryId,
    enabled: bool,
) -> Result<Result<(), DictionaryNotFound>> {
    let result = sqlx::query!(
        "UPDATE dictionaries
        SET enabled = $1
        WHERE id = $2",
        enabled,
        dictionary_id.0
    )
    .execute(db)
    .await
    .context("failed to delete record")?;
    Ok(if result.rows_affected() > 0 {
        Ok(())
    } else {
        Err(DictionaryNotFound)
    })
}
