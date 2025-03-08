use {
    anyhow::{Context as _, Result, bail},
    futures::{StreamExt, TryStreamExt},
    serde::{Deserialize, Serialize},
    sqlx::{Pool, Row, Sqlite},
    std::io,
    wordbase::{DictionaryId, Record, RecordKind, Term, protocol::LookupResponse},
};

pub fn serialize(
    value: &impl Serialize,
    writer: impl io::Write,
) -> Result<(), rmp_serde::encode::Error> {
    value.serialize(&mut rmp_serde::Serializer::new(writer))
}

pub fn deserialize<'a, T: Deserialize<'a>>(buf: &'a [u8]) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(buf)
}

// TODO: make this return a stream when async iterators are stabilized
pub async fn lookup(
    db: &Pool<Sqlite>,
    text: &str,
    include: &[RecordKind],
    exclude: &[RecordKind],
) -> Result<Vec<LookupResponse>> {
    macro_rules! deserialize_record_kinds {
        ( $record:expr, $($kind:ident),* $(,)? ) => {{
            #[expect(
                non_upper_case_globals,
                reason = "cannot capitalize ident in macro invocation"
            )]
            mod discrim {
                use super::RecordKind;

                $(pub const $kind: u16 = RecordKind::$kind as u16;)*
            }

            match u16::try_from($record.kind) {
                $(
                    Ok(discrim::$kind) => {
                        let record = deserialize(&($record.data))
                            .with_context(|| format!("failed to deserialize {} record", stringify!($kind)))?;
                        Record::$kind(record)
                    }
                )*
                _ => bail!("invalid record kind {}", $record.kind),
            }
        }};
    }

    let mut query = sqlx::QueryBuilder::new(
        "SELECT source, headword, reading, kind, data
        FROM term t
        LEFT JOIN dictionary
            ON t.source = dictionary.id
        WHERE
            dictionary.enabled = TRUE",
    );

    let sql = {
        let include = if include.is_empty() {
            String::new()
        } else {
            let include = (0..include.len())
                .map(|i| format!("${}", 2 + i))
                .collect::<Vec<_>>()
                .join(", ");
            format!("AND kind IN ({})", include)
        };

        let exclude = if exclude.is_empty() {
            String::new()
        } else {
            let exclude = (0..exclude.len())
                .map(|i| format!("${}", 2 + include.len() + i))
                .collect::<Vec<_>>()
                .join(", ");
            format!("AND kind NOT IN ({})", exclude)
        };

        format!(
            "SELECT source, headword, reading, kind, data
            FROM term t
            LEFT JOIN dictionary
                ON t.source = dictionary.id
            WHERE
                dictionary.enabled = TRUE
                AND (headword = $1 OR reading = $1)
                {include}
                {exclude}",
        )
    };

    let mut query = sqlx::query(&sql).bind(text);
    for record_kind in include {
        query = query.bind(*record_kind as u16);
    }
    for record_kind in exclude {
        query = query.bind(*record_kind as u16);
    }

    query
        .bind(text)
        .fetch(db)
        .map(|record| {
            struct QueryRecord {
                source: i64,
                headword: String,
                reading: Option<String>,
                kind: i64,
                data: Vec<u8>,
            }

            let row = record.context("failed to fetch record")?;
            let record = QueryRecord {
                source: row.get(0),
                headword: row.get(1),
                reading: row.get(2),
                kind: row.get(3),
                data: row.get(4),
            };

            let source = DictionaryId(record.source);
            let term = Term {
                headword: record.headword,
                reading: record.reading,
            };

            let record = deserialize_record_kinds! {
                record,
                Glossary,
                Frequency,
                JpPitch,
            };

            Ok(LookupResponse {
                source,
                term,
                record,
            })
        })
        .try_collect()
        .await
}
