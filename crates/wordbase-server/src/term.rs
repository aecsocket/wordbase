use {
    anyhow::{Context as _, Result, bail},
    futures::{Stream, StreamExt},
    serde::{Deserialize, Serialize},
    sqlx::{Pool, Sqlite},
    std::{io, pin::Pin},
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

pub async fn lookup<'q>(
    db: &'q Pool<Sqlite>,
    query: &'q str,
    include: &[RecordKind],
    exclude: &[RecordKind],
) -> Pin<Box<impl Stream<Item = Result<LookupResponse>> + 'q>> {
    macro_rules! deserialize_record_kinds {
        ( $record:expr, $($kind:ident),* $(,)? ) => {{
            mod discrim {
                use super::RecordKind;

                $(
                    #[expect(
                        non_upper_case_globals,
                        reason = "cannot capitalize ident in macro invocation"
                    )]
                    pub const $kind: u16 = RecordKind::$kind as u16;
                )*
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

    Box::pin(
        sqlx::query!(
            "SELECT source, headword, reading, kind, data
            FROM term t
            LEFT JOIN dictionary
                ON t.source = dictionary.id
            WHERE
                dictionary.enabled = TRUE
                AND (headword = $1 OR reading = $1)",
            query
        )
        .fetch(db)
        .map(|record| {
            let record = record.context("failed to fetch record")?;

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
        }),
    );

    futures::stream::empty()
}
