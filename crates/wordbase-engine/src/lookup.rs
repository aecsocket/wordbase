use {
    crate::{Engine, db},
    anyhow::{Context, Result, bail},
    futures::{StreamExt, TryStreamExt},
    sqlx::{QueryBuilder, Row},
    std::borrow::Borrow,
    wordbase::{
        DictionaryId, Record, RecordId, RecordKind, RecordLookup, Term, TermKind, for_kinds,
    },
};

impl Engine {
    pub async fn lookup_lemma(
        &self,
        lemma: &str,
        record_kinds: impl IntoIterator<Item = impl Borrow<RecordKind>>,
    ) -> Result<Vec<RecordLookup>> {
        let mut query = QueryBuilder::new(
            r#"SELECT
                record.id,
                record.source,
                record.kind,
                record.data,
                term.text as "term_text",
                term.kind AS "term_kind"
            FROM record
            INNER JOIN dictionary ON record.source = dictionary.id
            INNER JOIN profile_enabled_dictionary ped ON dictionary.id = ped.dictionary
            INNER JOIN config ON ped.profile = config.current_profile
            INNER JOIN term ON term.record = record.id
            WHERE term.text = "#,
        );
        query.push_bind(lemma);
        query.push("AND term.kind IN (");
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
                id: i64,
                source: i64,
                kind: i64,
                data: Vec<u8>,
                term_text: String,
                term_kind: i64,
            }

            let row = record.context("failed to fetch record")?;
            let record = QueryRecord {
                id: row.get(0),
                source: row.get(1),
                kind: row.get(2),
                data: row.get(3),
                term_text: row.get(4),
                term_kind: row.get(5),
            };

            let source = DictionaryId(record.source);
            let term_kind = match record.term_kind {
                0 => TermKind::Headword,
                1 => TermKind::Reading,
                _ => bail!("invalid term kind `{}`", record.term_kind),
            };
            let term = Term::new(term_kind, record.term_text)
                .context("database contained empty term - this should never happen due to SQL contraints")?;
            let record_id = RecordId(record.id);

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

            let record = for_kinds!(deserialize_record);

            Ok(RecordLookup {
                source,
                term,
                record_id,
                record,
            })
        })
        .try_collect()
        .await
    }
}
