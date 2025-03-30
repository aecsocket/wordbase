use {
    crate::{Engine, db},
    anyhow::{Context, Result, bail},
    futures::{StreamExt, TryStreamExt},
    sqlx::{QueryBuilder, Row},
    std::borrow::Borrow,
    wordbase::{DictionaryId, Record, RecordKind, RecordLookup, Term, for_kinds},
};

impl Engine {
    pub async fn lookup_lemma(
        &self,
        lemma: &str,
        record_kinds: impl IntoIterator<Item = impl Borrow<RecordKind>>,
    ) -> Result<Vec<RecordLookup>> {
        let mut query = QueryBuilder::new(
            "SELECT
                record.source,
                record.kind,
                record.data,
                term_record.headword,
                term_record.reading
            FROM record
            INNER JOIN dictionary ON record.source = dictionary.id
            INNER JOIN profile_enabled_dictionary ped ON dictionary.id = ped.dictionary
            INNER JOIN config ON ped.profile = config.current_profile
            INNER JOIN term_record ON term_record.record = record.id
            WHERE (term_record.headword = ",
        );
        query.push_bind(lemma);
        query.push("OR term_record.reading = ");
        query.push_bind(lemma);
        query.push(") AND record.kind IN (");
        {
            let mut query = query.separated(", ");
            for record_kind in record_kinds {
                query.push_bind(*record_kind.borrow() as u16);
            }
            query.push_unseparated(") ");
        }
        query.push("ORDER BY dictionary.position");

        query.build().fetch(&self.db).map(|record| {
            struct QueryRecord {
                source: i64,
                kind: i64,
                data: Vec<u8>,
                headword: String,
                reading: String,
            }

            let record = record.context("failed to fetch record")?;
            let record = QueryRecord {
                source: record.get(0),
                kind: record.get(1),
                data: record.get(2),
                headword: record.get(3),
                reading: record.get(4),
            };

            macro_rules! deserialize_record { ($($dict_kind:ident($dict_path:ident) { $($record_kind:ident),* $(,)? }),* $(,)?) => { paste::paste! {{
                #[allow(
                    non_upper_case_globals,
                    reason = "cannot capitalize ident in macro invocation"
                )]
                mod discrim {
                    use super::RecordKind;

                    $($(
                    pub const [< $dict_kind $record_kind >]: u32 = RecordKind::[< $dict_kind $record_kind >] as u32;
                    )*)*
                }

                match u32::try_from(record.kind) {
                    $($(
                        Ok(discrim::[< $dict_kind $record_kind >]) => {
                        let record = db::deserialize(&record.data)
                            .with_context(|| format!("failed to deserialize {} record", stringify!([< $dict_kind $record_kind >])))?;
                        Record::[< $dict_kind $record_kind >](record)
                    }
                    )*)*
                    _ => bail!("invalid record kind {}", record.kind),
                }
            }}}}

            Ok(RecordLookup {
                source: DictionaryId(record.source),
                term: Term::new(record.headword, record.reading).context("fetched empty term")?,
                record: for_kinds!(deserialize_record),
            })
        })
        .try_collect()
        .await
    }
}
