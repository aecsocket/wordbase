use {
    crate::{Engine, db},
    anyhow::{Context, Result, bail},
    futures::{Stream, StreamExt, TryStreamExt, stream},
    itertools::Itertools,
    std::borrow::Borrow,
    wordbase::{DictionaryId, FrequencyValue, Record, RecordKind, RecordLookup, Term, for_kinds},
};

impl Engine {
    pub fn lookup_lemma(
        &self,
        lemma: impl AsRef<str>,
        record_kinds: &[impl Borrow<RecordKind>],
    ) -> impl Stream<Item = Result<RecordLookup>> {
        stream::once(async move {
            let lemma = lemma.as_ref();
            // we do a hack where we turn `record_kinds` into a JSON array of ints
            // because SQLite doesn't support placeholders of tuples or arrays
            let record_kinds = format!(
                "[{}]",
                record_kinds
                    .iter()
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
                    profile_frequency.mode AS 'profile_frequency_mode?',
                    profile_frequency.value AS 'profile_frequency_value?',
                    source_frequency.mode AS 'source_frequency_mode?',
                    source_frequency.value AS 'source_frequency_value?'
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

                -- join on profile-global frequency information, for the `ORDER BY` below
                LEFT JOIN frequency profile_frequency ON (
                    -- only use frequency info from the currently selected sorting dict in this profile
                    profile_frequency.source = (
                        SELECT sorting_dictionary FROM profile
                        WHERE id = config.current_profile
                    )
                    AND profile_frequency.headword = term_record.headword
                    AND profile_frequency.reading = term_record.reading
                )

                -- join on frequency information for this source
                LEFT JOIN frequency source_frequency ON (
                    source_frequency.source = record.source
                    AND source_frequency.headword = term_record.headword
                    AND source_frequency.reading = term_record.reading
                )

                -- only include records for the given record kinds
                WHERE kind IN (SELECT value FROM json_each($2))

                ORDER BY
                    -- user-specified dictionary sorting position always takes priority
                    dictionary.position,
                    -- put entries without an explicit frequency value last
                    CASE
                        WHEN profile_frequency.mode IS NULL THEN 1
                        ELSE 0
                    END,
                    -- sort by profile-global frequency info
                    CASE
                        -- frequency rank
                        WHEN profile_frequency.mode = 0 THEN  profile_frequency.value
                        -- frequency occurrence
                        WHEN profile_frequency.mode = 1 THEN -profile_frequency.value
                        ELSE 0
                    END,
                    -- sort by source-specific frequency info
                    CASE
                        WHEN source_frequency.mode = 0 THEN  source_frequency.value
                        WHEN source_frequency.mode = 1 THEN -source_frequency.value
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

                Ok(RecordLookup {
                    bytes_scanned: lemma.len(),
                    source: DictionaryId(record.source),
                    term: Term::new(record.headword, record.reading)
                        .context("fetched empty term")?,
                    record: for_kinds!(deserialize_record),
                    profile_sorting_frequency: to_frequency_value(
                        record.profile_frequency_mode,
                        record.profile_frequency_value,
                    ),
                    source_sorting_frequency: to_frequency_value(
                        record.source_frequency_mode,
                        record.source_frequency_value,
                    ),
                })
            });

            match result.try_collect::<Vec<_>>().await {
                Ok(results) => stream::iter(results.into_iter().map(anyhow::Ok)).left_stream(),
                Err(err) => stream::once(async move { Err(err) }).right_stream(),
            }
        })
        .flatten()
    }

    pub fn lookup<'a>(
        &'a self,
        context: &'a str,
        cursor: usize,
        record_kinds: &[impl Borrow<RecordKind>],
    ) -> impl Stream<Item = Result<RecordLookup>> {
        // TODO: languages with words separated by e.g. spaces need a different strategy
        let Some((_, query)) = context.split_at_checked(cursor) else {
            return stream::once(
                async move { bail!("cursor is not on a UTF-8 character boundary") },
            )
            .left_stream();
        };

        self.deinflect(query)
            .flat_map(|deinflection| {
                self.lookup_lemma(deinflection.lemma, record_kinds)
                    .map(move |result| {
                        result.map(|record| RecordLookup {
                            bytes_scanned: deinflection.scan_len,
                            ..record
                        })
                    })
            })
            .right_stream()
    }
}

fn to_frequency_value(mode: Option<i64>, value: Option<i64>) -> Option<FrequencyValue> {
    match (mode, value) {
        (Some(0), Some(value)) => Some(FrequencyValue::Rank(value)),
        (Some(1), Some(value)) => Some(FrequencyValue::Occurrence(value)),
        _ => None,
    }
}
