use {
    crate::{Engine, db},
    anyhow::{Context, Result, bail},
    futures::{Stream, StreamExt, TryStreamExt, stream},
    itertools::Itertools,
    std::borrow::Borrow,
    wordbase::{DictionaryId, FrequencyValue, Record, RecordKind, RecordLookup, Term, for_kinds},
};

#[derive(Debug, Clone)]
pub struct LemmaLookup {
    pub source: DictionaryId,
    pub term: Term,
    pub record: Record,
    pub frequency: Option<FrequencyValue>,
}

impl Engine {
    pub fn lookup_lemma(
        &self,
        lemma: impl AsRef<str>,
        record_kinds: impl IntoIterator<Item = impl Borrow<RecordKind>>,
    ) -> impl Stream<Item = Result<LemmaLookup>> {
        stream::once(async move {
            let lemma = lemma.as_ref();
            // we do a hack where we turn `record_kinds` into a JSON array of ints
            // because SQLite doesn't support placeholders of tuples or arrays
            let record_kinds = format!(
                "[{}]",
                record_kinds
                    .into_iter()
                    .map(|kind| *kind.borrow() as u32)
                    .format(",")
            );

            let result = sqlx::query!(
                "SELECT
                    record.source,
                    record.kind,
                    record.data,
                    term_record.headword,
                    term_record.reading,
                    frequency.mode AS 'frequency_mode?',
                    frequency.value AS 'frequency_value?'
                FROM record
                INNER JOIN dictionary ON record.source = dictionary.id

                -- make sure the dictionary we're getting this record from is enabled
                INNER JOIN config
                INNER JOIN profile_enabled_dictionary ped
                    ON (ped.profile = config.current_profile AND ped.dictionary = dictionary.id)

                -- find which terms reference this record, either through the headword or reading
                INNER JOIN term_record ON (
                    term_record.record = record.id
                    AND (term_record.headword = $1 OR term_record.reading = $1)
                )

                -- join on frequency information, for the `ORDER BY` below
                LEFT JOIN frequency ON (
                    -- only use frequency info from the currently selected sorting dict in this profile
                    frequency.source = (
                        SELECT sorting_dictionary FROM profile
                        WHERE id = config.current_profile
                    )
                    AND frequency.headword = term_record.headword
                    AND frequency.reading = term_record.reading
                )

                -- only include records for the given record kinds
                WHERE kind IN (SELECT value FROM json_each($2))

                ORDER BY
                    -- user-specified dictionary sorting position always takes priority
                    dictionary.position,
                    CASE
                        -- put entries without an explicit frequency value last
                        WHEN frequency.mode IS NULL THEN 1
                        ELSE 0
                    END,
                    CASE
                        -- frequency rank
                        WHEN frequency.mode = 0 THEN  frequency.value
                        -- frequency occurrence
                        WHEN frequency.mode = 1 THEN -frequency.value
                        ELSE 0
                    END",
                lemma,
                record_kinds
            )
            .fetch(&self.db)
            .map(|record| {
                let record = record.context("failed to fetch record")?;

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

                Ok(LemmaLookup {
                    source: DictionaryId(record.source),
                    term: Term::new(record.headword, record.reading)
                        .context("fetched empty term")?,
                    record: for_kinds!(deserialize_record),
                    frequency: match (
                        record.frequency_mode,
                        record.frequency_value.map(u64::try_from),
                    ) {
                        (Some(0), Some(Ok(value))) => Some(FrequencyValue::Rank(value)),
                        (Some(1), Some(Ok(value))) => Some(FrequencyValue::Occurrence(value)),
                        _ => None,
                    },
                })
            });

            match result.try_collect::<Vec<_>>().await {
                Ok(results) => stream::iter(results.into_iter().map(anyhow::Ok)).boxed(),
                Err(err) => stream::once(async move { Err(err) }).boxed(),
            }
        })
        .flatten()
    }

    pub fn lookup(
        &self,
        query: impl AsRef<str>,
        record_kinds: &[RecordKind],
    ) -> impl Stream<Item = Result<RecordLookup>> {
        self.deinflect(query)
            .map(move |lemma| {
                self.lookup_lemma(lemma, record_kinds).map(|result| {
                    result.map(|lookup| RecordLookup {
                        bytes_scanned: 0,
                        source: lookup.source,
                        term: lookup.term,
                        record: lookup.record,
                        frequency: lookup.frequency,
                    })
                })
            })
            .flatten()

        // let x = self
        //     .deinflect(query)
        //     .await
        //     .map(|lemma| {
        //         stream::once(async move {
        //             self.lookup_lemma(lemma, record_kinds)
        //                 .await
        //                 .map(move |result| {
        //                     result.map(move |lookup| RecordLookup {
        // bytes_scanned: 0,
        // source: lookup.source,
        // term: lookup.term,
        // record: lookup.record,
        // frequency: lookup.frequency,
        //                     })
        //                 })
        //         })
        //     })
        //     .flatten()
        //     .collect::<Vec<_>>()
        //     .await;

        // Ok(self.lookup_lemma(query, record_kinds).await.map(|result| {
        //     result.map(
        //         |LemmaLookup {
        //              source,
        //              term,
        //              record,
        //              frequency,
        //          }| RecordLookup {
        //             bytes_scanned: 0,
        //             source,
        //             term,
        //             record,
        //             frequency,
        //         },
        //     )
        // }))
    }
}
