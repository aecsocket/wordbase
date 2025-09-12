use {
    crate::{
        IndexSet, Wordbase, db,
        deinflect::{self, Deinflection, Deinflector},
    },
    anyhow::{Context, Result, bail},
    foldhash::{HashSet, HashSetExt},
    futures::{StreamExt, TryStreamExt, stream::FuturesOrdered},
    itertools::Itertools as _,
    std::iter,
    wordbase_api::{
        DictionaryId, FrequencyValue, NoHeadwordOrReading, ProfileId, Record, RecordEntry,
        RecordId, RecordKind, Span, Term, for_kinds,
    },
};

#[derive(Debug)]
#[cfg_attr(feature = "uniffi", derive(uniffi::Object))]
pub struct Lookups {
    lindera: deinflect::Lindera,
}

impl Lookups {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            lindera: deinflect::Lindera::new()
                .await
                .context("failed to create deinflector `Lindera`")?,
        })
    }

    pub fn deinflect<'a>(&'a self, sentence: &'a str, cursor: usize) -> IndexSet<Deinflection<'a>> {
        iter::empty()
            // TODO: disable deinflectors based on language
            .chain(deinflect::Identity.deinflect(sentence, cursor))
            .chain(self.lindera.deinflect(sentence, cursor))
            .chain(deinflect::Latin.deinflect(sentence, cursor))
            .inspect(|deinflect| {
                debug_assert!(
                    sentence.get(deinflect.span.clone()).is_some(),
                    "text = {sentence:?}, cursor = {cursor}, span = {:?}",
                    deinflect.span
                );
            })
            .sorted_by_key(|deinflect| deinflect.span.start)
            .collect::<IndexSet<_>>()
    }

    pub async fn lookup_lemma(
        &self,
        engine: &Wordbase,
        profile_id: ProfileId,
        lemma: impl AsRef<str> + Send + Sync,
    ) -> Result<Vec<RecordEntry>> {
        let lemma = lemma.as_ref();
        let query = sqlx::query!(
            "
            -- use a CTE to get results for all records matching the headword and reading,
            -- instead of `WHERE headword = $2 OR reading = $2`
            -- this makes it clear to the query planner that we want to use these indexes:
            -- - `record(headword, source, kind)`
            -- - `record(reading, source, kind)`
            --
            -- otherwise, the query planner might use the `record(source)` index,
            -- which would kill performance
            WITH base AS (
                SELECT headword, reading, record FROM term_record
                INDEXED BY term_record_query_headword
                WHERE headword = $2

                -- we can get away with `UNION ALL` here,
                -- because we don't guarantee to callers that there won't be duplicate records
                UNION ALL

                SELECT headword, reading, record FROM term_record
                INDEXED BY term_record_query_reading
                WHERE reading = $2
            )
            SELECT
                record.id,
                record.source,
                record.kind,
                record.data,
                base.headword,
                base.reading,
                profile_frequency.mode AS 'profile_frequency_mode?',
                profile_frequency.value AS 'profile_frequency_value?',
                source_frequency.mode AS 'source_frequency_mode?',
                source_frequency.value AS 'source_frequency_value?'
            FROM record
            JOIN base ON record.id = base.record

            -- make sure the dictionary we're getting this record from is enabled
            INNER JOIN dictionary ON record.source = dictionary.id
            INNER JOIN profile_enabled_dictionary ped
                ON (ped.profile = $1 AND ped.dictionary = dictionary.id)

            -- join on profile-global frequency information, for the `ORDER BY` below
            LEFT JOIN frequency profile_frequency ON (
                -- only use frequency info from the currently selected sorting dict in this profile
                profile_frequency.source = (
                    SELECT sorting_dictionary FROM profile
                    WHERE id = $1
                )
                AND profile_frequency.headword = base.headword
                AND profile_frequency.reading = base.reading
            )

            -- join on frequency information for this source
            LEFT JOIN frequency source_frequency ON (
                source_frequency.source = record.source
                AND source_frequency.headword = base.headword
                AND source_frequency.reading = base.reading
            )

            ORDER BY
                CASE
                    -- prioritize results where both the headword and reading match the lemma
                    -- e.g. if you typed あらゆる:
                    -- - the first results would be for the kana あらゆる
                    -- - then the kanji like 汎ゆる
                    WHEN base.reading = $2 AND base.headword = $2 THEN 0
                    -- then prioritize results where at least the reading or headword are an exact \
             match
                    -- e.g. in 念じる, usually 念ずる comes up first
                    -- but this is obviously a different reading
                    -- so we want to prioritize 念じる
                    WHEN base.reading = $2 OR base.headword = $2 THEN 1
                    -- all other results at the end
                    ELSE 2
                END,
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
            profile_id.0,
            lemma,
        );

        let result = query
            .fetch(&engine.db)
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

                    (|| {
                        match u32::try_from(record.kind) {
                            $($(
                            Ok(discrim::[< $dict_kind $record_kind >]) => {
                                let record = db::deserialize(&record.data)
                                    .with_context(|| format!("failed to deserialize as {}", stringify!([< $dict_kind $record_kind >])))?;
                                anyhow::Ok(Record::[< $dict_kind $record_kind >](record))
                            }
                            )*)*
                            _ => bail!("invalid record kind {}", record.kind),
                        }
                    })()
                }}}}

                let source = DictionaryId(record.source);
                let term = Term::from_parts(record.headword, record.reading)
                    .ok_or(NoHeadwordOrReading)?;

                let typed_record = for_kinds!(deserialize_record)
                    .with_context(|| {
                        format!(
                            "failed to deserialize record {term:?} from dictionary {:?} ({source:?})",
                            engine.dictionaries().get(&source).map_or("?", |dict| dict.meta.name.as_str())
                        )
                    })?;

                Ok(RecordEntry {
                    span_bytes: (0..lemma.len()).try_into()
                        .context("byte span too large")?,
                    span_chars: (0..lemma.chars().count()).try_into()
                        .context("char span too large")?,
                    source: DictionaryId(record.source),
                    record_id: RecordId(record.id),
                    term,
                    record: typed_record,
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
        result.try_collect::<Vec<_>>().await
    }

    pub async fn lookup<'a>(
        &'a self,
        engine: &Wordbase,
        profile_id: ProfileId,
        sentence: &'a str,
        cursor: usize,
    ) -> Result<Vec<RecordEntry>> {
        let mut records = Vec::new();
        let mut seen_record_ids = HashSet::new();
        let deinflections = self.deinflect(sentence, cursor);
        let mut lookup_tasks = deinflections
            .iter()
            .map(|deinflection| async move {
                self.lookup_lemma(engine, profile_id, &deinflection.lemma)
                    .await
                    .map(|entries| (deinflection, entries))
            })
            .collect::<FuturesOrdered<_>>();
        while let Some((deinflection, entries)) = lookup_tasks.try_next().await? {
            let span_bytes =
                Span::try_from(deinflection.span.clone()).context("byte span too large")?;

            let span_chars_start = sentence
                .get(..deinflection.span.start)
                .context("deinflection span start is invalid")?
                .chars()
                .count();
            let span_chars_len = sentence
                .get(deinflection.span.clone())
                .context("deinflection span is invalid")?
                .chars()
                .count();
            let span_chars = Span::try_from(span_chars_start..(span_chars_start + span_chars_len))
                .context("char span too large")?;

            for record in entries {
                if !seen_record_ids.insert(record.record_id) {
                    continue;
                }

                records.push(RecordEntry {
                    span_bytes,
                    span_chars,
                    ..record
                });
            }
        }

        Ok(records)
    }
}

fn to_frequency_value(mode: Option<i64>, value: Option<i64>) -> Option<FrequencyValue> {
    match (mode, value) {
        (Some(0), Some(value)) => Some(FrequencyValue::Rank(value)),
        (Some(1), Some(value)) => Some(FrequencyValue::Occurrence(value)),
        _ => None,
    }
}

#[cfg(feature = "uniffi")]
const _: () = {
    use crate::{FfiResult, Wordbase};

    #[derive(uniffi::Record)]
    pub struct Deinflection {
        pub lemma: String,
        pub span: Span,
    }

    #[uniffi::export(async_runtime = "tokio")]
    impl Lookups {
        #[uniffi::method(name = "deinflect")]
        pub fn ffi_deinflect(&self, sentence: &str, cursor: u64) -> FfiResult<Vec<Deinflection>> {
            let cursor = usize::try_from(cursor).context("cursor too large")?;
            Ok(self
                .deinflect(sentence, cursor)
                .into_iter()
                .map(|deinflect| {
                    anyhow::Ok(Deinflection {
                        lemma: deinflect.lemma.into_owned(),
                        span: deinflect.span.try_into().context("span too large")?,
                    })
                })
                .collect::<Result<Vec<Deinflection>, _>>()?)
        }

        #[uniffi::method(name = "lookup")]
        pub async fn ffi_lookup<'a>(
            &'a self,
            engine: &Wordbase,
            profile_id: ProfileId,
            sentence: &'a str,
            cursor: u64,
        ) -> FfiResult<Vec<RecordEntry>> {
            let cursor = usize::try_from(cursor).context("cursor too large")?;
            Ok(self.lookup(engine, profile_id, sentence, cursor).await?)
        }
    }
};

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::deinflect::{Deinflection, sentence},
    };

    #[tokio::test]
    async fn deinflections_starting_earlier_first() {
        // when a user clicks in the middle of a word, some later deinflectors
        // may end up producing lemmas which start *earlier* than the earlier
        // deinflectors. for example, in "アルコール", if you click in between
        // "アル" and "コール", we will try deinflecting "コール" first, and
        // then "アルコール". in this case, the text highlighted to the user
        // will be e.g. "アルコール", but the first results will be for "コール".
        //
        // to avoid this, we sort the deinflections by whichever ones have spans
        // that start the earliest. this test ensures that.
        let lookups = Lookups::new().await.unwrap();
        let (text, cursor1, cursor2) = sentence!("手を" / "アル" / "コールで消毒");
        let mut lemmas = lookups.deinflect(text, cursor2).into_iter();
        assert_eq!(
            lemmas.next().unwrap(),
            Deinflection::new(cursor1, "アルコール", "アルコール")
        );
        assert!(lemmas.next().is_some());
    }
}
