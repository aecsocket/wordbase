use {
    crate::{Engine, db},
    anyhow::{Context, Result, bail},
    futures::{Stream, StreamExt},
    sqlx::{QueryBuilder, Row},
    wordbase::{
        DictionaryId, Record, RecordKind, Term, for_record_kinds, protocol::LookupResponse,
    },
};

impl Engine {
    pub async fn lookup(
        &self,
        text: impl Into<String>,
        record_kinds: impl IntoIterator<Item = RecordKind>,
    ) -> impl Stream<Item = Result<LookupResponse>> {
        let text = text.into();
        // TODO: lemmatization
        let lemma = &text;

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
                query.push_bind(record_kind as u16);
            }
            query.push_unseparated(") ");
        }
        query.push("ORDER BY dictionary.position");

        let results = query.build().fetch(&self.db).map(|record| {
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
                        let record = db::deserialize(&record.data)
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
}
