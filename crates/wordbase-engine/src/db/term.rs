use {
    anyhow::{Context as _, Result, bail},
    futures::{Stream, StreamExt},
    serde::{Deserialize, Serialize},
    sqlx::{Executor, QueryBuilder, Row, Sqlite},
    std::io,
    wordbase::{
        DictionaryId, Record, RecordKind, RecordType, Term, for_record_kinds,
        protocol::LookupResponse,
    },
};

fn serialize(
    value: &impl Serialize,
    writer: impl io::Write,
) -> Result<(), rmp_serde::encode::Error> {
    value.serialize(&mut rmp_serde::Serializer::new(writer))
}

fn deserialize<'a, T: Deserialize<'a>>(buf: &'a [u8]) -> Result<T, rmp_serde::decode::Error> {
    rmp_serde::from_slice(buf)
}

pub async fn insert<'e, 'c: 'e, E, R>(
    executor: E,
    source: DictionaryId,
    term: &Term,
    record: &R,
    scratch: &mut Vec<u8>,
) -> Result<()>
where
    E: 'e + Executor<'c, Database = Sqlite>,
    R: RecordType,
{
    scratch.clear();
    serialize(record, &mut *scratch).context("failed to serialize record")?;

    let headword = term.headword();
    let reading = term.reading();
    let data = &scratch[..];
    sqlx::query!(
        "INSERT INTO term (source, headword, reading, kind, data)
        VALUES ($1, $2, $3, $4, $5)",
        source.0,
        headword,
        reading,
        R::KIND as u16,
        data
    )
    .execute(executor)
    .await?;
    Ok(())
}

pub async fn lookup<'c, 'e: 'c, E>(
    executor: E,
    lemma: &str,
    record_kinds: &[RecordKind],
) -> impl Stream<Item = Result<LookupResponse>> + use<E>
where
    E: 'e + Executor<'c, Database = Sqlite>,
{
    let mut query = QueryBuilder::new(
        "SELECT source, headword, reading, kind, data
        FROM term
        INNER JOIN dictionary ON term.source = dictionary.id
        INNER JOIN profile_enabled_dictionary ped ON dictionary.id = ped.dictionary
        INNER JOIN config ON ped.profile = config.current_profile
        WHERE
            (term.headword = ",
    );
    query.push_bind(lemma);
    query.push(" OR term.reading = ");
    query.push_bind(lemma);
    query.push(") AND term.kind IN (");
    {
        let mut query = query.separated(", ");
        for record_kind in record_kinds {
            query.push_bind(*record_kind as u16);
        }
        query.push_unseparated(") ");
    }
    query.push("ORDER BY dictionary.position");

    let results = query.build().fetch(executor).map(|record| {
        struct QueryRecord {
            source: i64,
            headword: Option<String>,
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
        let term = Term::from_pair(record.headword, record.reading)
            .context("found record where both headword and reading are null")?;

        macro_rules! deserialize_record { ($($kind:ident($data_ty:path)),* $(,)?) => {{
            #[allow(
                non_upper_case_globals,
                reason = "cannot capitalize ident in macro invocation"
            )]
            mod discrim {
                use super::RecordKind;

                $(pub const $kind: u16 = RecordKind::$kind as u16;)*
            }

            match u16::try_from(record.kind) {
                $(Ok(discrim::$kind) => {
                    let record = deserialize(&record.data)
                        .with_context(|| format!("failed to deserialize {} record", stringify!($kind)))?;
                    Record::$kind(record)
                })*
                _ => bail!("invalid record kind {}", record.kind),
            }
        }}}

        let record = for_record_kinds!(deserialize_record);

        Ok(LookupResponse {
            source,
            term,
            record,
        })
    });

    // TODO: we need async iterator generators!
    let results = results.collect::<Vec<_>>().await;
    futures::stream::iter(results)
}
