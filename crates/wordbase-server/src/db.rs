use anyhow::{Context as _, Result, bail};
use futures::{StreamExt, TryStreamExt};
use sqlx::{Pool, Sqlite};
use wordbase::{
    protocol::DictionaryNotFound,
    schema::{Dictionary, DictionaryId, Frequency, Glossary, LookupInfo, Pitch, Term},
};

// keep this up to date with `setup_db.sql`
pub mod data_kind {
    pub const GLOSSARY: u8 = 1;
    pub const FREQUENCY: u8 = 2;
    pub const PITCH: u8 = 3;
}

pub async fn lookup(db: &Pool<Sqlite>, lemma: String) -> Result<LookupInfo> {
    let mut info = LookupInfo {
        lemma: lemma.clone(),
        ..Default::default()
    };
    let mut records = sqlx::query!(
        "SELECT source, expression, reading, data_kind, data
        FROM terms t
        LEFT JOIN dictionaries
            ON t.source = dictionaries.id
        WHERE
            dictionaries.enabled = TRUE
            AND (expression = $1 OR reading = $1)",
        lemma
    )
    .fetch(db);
    while let Some(record) = records.next().await {
        let record = record.context("failed to fetch record")?;
        let source = DictionaryId(record.source);
        let term = Term {
            expression: record.expression,
            reading: record.reading,
        };

        // this will cause duplicate term entries
        // we strip these out later
        info.terms.push((source, term.clone()));

        match u8::try_from(record.data_kind) {
            Ok(data_kind::GLOSSARY) => {
                let data = postcard::from_bytes::<Glossary>(&record.data)
                    .context("failed to deserialize glossary data")?;
                info.glossaries.push((source, term, data));
            }
            Ok(data_kind::FREQUENCY) => {
                let data = postcard::from_bytes::<Frequency>(&record.data)
                    .context("failed to deserialize frequency data")?;
                info.frequencies.push((source, term, data));
            }
            Ok(data_kind::PITCH) => {
                let data = postcard::from_bytes::<Pitch>(&record.data)
                    .context("failed to deserialize pitch data")?;
                info.pitches.push((source, term, data));
            }
            _ => bail!("invalid data kind {}", record.data_kind),
        }
    }

    info.terms.dedup();
    Ok(info)
}

pub async fn list_dictionaries(db: &Pool<Sqlite>) -> Result<Vec<Dictionary>> {
    sqlx::query!(
        "SELECT id, title, revision, enabled
        FROM dictionaries"
    )
    .fetch(db)
    .map(|record| {
        let record = record.context("failed to fetch record")?;
        anyhow::Ok(Dictionary {
            id: DictionaryId(record.id),
            title: record.title,
            revision: record.revision,
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
