use {
    anyhow::{Context as _, Result, bail},
    futures::{StreamExt, TryStreamExt},
    serde::{Deserialize, Serialize},
    sqlx::{Pool, QueryBuilder, Row, Sqlite},
    std::io,
    wordbase::{
        DictionaryId, Record, RecordKind, Term, for_record_kinds, protocol::LookupResponse,
    },
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
    record_kinds: &[RecordKind],
) -> Result<Vec<LookupResponse>> {
    let mut query = QueryBuilder::new(
        "SELECT source, headword, reading, kind, data
        FROM term t
        LEFT JOIN dictionary
            ON t.source = dictionary.id
        WHERE
            dictionary.enabled = TRUE
            AND (headword = ",
    );
    query.push_bind(text);
    query.push(" OR reading = ");
    query.push_bind(text);
    query.push(") AND kind IN (");
    {
        let mut query = query.separated(", ");
        for record_kind in record_kinds {
            query.push_bind(*record_kind as u16);
        }
        query.push_unseparated(")");
    }

    query
        .build()
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

            macro_rules! deserialize_record { ( $($kind:ident($data_ty:path))* ) => {{
                #[allow(
                    non_upper_case_globals,
                    reason = "cannot capitalize ident in macro invocation"
                )]
                mod discrim {
                    use super::RecordKind;

                    $(pub const $kind: u16 = RecordKind::$kind as u16;)*
                }

                match u16::try_from(record.kind) {
                    $(
                        Ok(discrim::$kind) => {
                            let record = deserialize(&record.data)
                                .with_context(|| format!("failed to deserialize {} record", stringify!(#kind)))?;
                            Record::$kind(record)
                        }
                    )*
                    _ => bail!("invalid record kind {}", record.kind),
                }
            }}}

            let record = for_record_kinds!(deserialize_record);

            Ok(LookupResponse {
                source,
                term,
                record,
            })
        })
        .try_collect()
        .await
}
