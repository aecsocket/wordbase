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
        let query = sqlx::query!(
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
            LEFT JOIN frequency ON (
                frequency.source = (SELECT sorting_dictionary FROM profile WHERE id = config.current_profile)
                AND (
                    (frequency.headword IS NOT NULL AND frequency.headword = term_record.headword)
                    OR
                    (frequency.reading IS NOT NULL AND frequency.reading = term_record.reading)
                )
            )
            WHERE
                (term_record.headword = $1 OR term_record.reading = $1)
                -- AND record.kind IN $2
            ORDER BY
                dictionary.position,
                CASE
                    WHEN frequency.mode = 0 THEN -frequency.value  -- occurrence mode
                    WHEN frequency.mode = 1 THEN  frequency.value  -- rank mode
                    ELSE 0
                END",
            lemma
        );

        /*
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
            LEFT JOIN frequency ON (
                frequency.source = (SELECT sorting_dictionary FROM profile WHERE id = config.current_profile)
                AND (
                    (frequency.headword IS NOT NULL AND frequency.headword = term_record.headword)
                    OR
                    (frequency.reading IS NOT NULL AND frequency.reading = term_record.reading)
                )
            )
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
        query.push(
            "ORDER BY
                CASE
                    WHEN
                dictionary.position",
        );*/

        query.fetch(&self.db).map(|record| {
            let record = record.context("failed to fetch record")?;
            /*
            struct QueryRecord {
                source: i64,
                kind: i64,
                data: Vec<u8>,
                headword: String,
                reading: String,
            }
            let record = QueryRecord {
                source: record.get(0),
                kind: record.get(1),
                data: record.get(2),
                headword: record.get(3),
                reading: record.get(4),
            };
             */

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
