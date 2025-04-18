use {
    crate::{Engine, db},
    anyhow::{Context, Result, bail},
    foldhash::{HashSet, HashSetExt},
    futures::{StreamExt, TryStreamExt},
    itertools::Itertools,
    std::borrow::Borrow,
    wordbase::{
        DictionaryId, FrequencyValue, ProfileId, Record, RecordId, RecordKind, RecordLookup, Term,
        for_kinds,
    },
};

impl Engine {
    pub async fn lookup_lemma(
        &self,
        profile_id: ProfileId,
        lemma: impl AsRef<str> + Send + Sync,
        record_kinds: &[impl Borrow<RecordKind> + Send + Sync],
    ) -> Result<Vec<RecordLookup>> {
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
                record.id,
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
            INNER JOIN profile_enabled_dictionary ped
                ON (ped.profile = $1 AND ped.dictionary = dictionary.id)

            -- find which terms reference this record, either through the headword or reading
            INNER JOIN term_record ON (
                term_record.record = record.id
                AND (term_record.headword = $2 OR term_record.reading = $2)
            )

            -- join on profile-global frequency information, for the `ORDER BY` below
            LEFT JOIN frequency profile_frequency ON (
                -- only use frequency info from the currently selected sorting dict in this profile
                profile_frequency.source = (
                    SELECT sorting_dictionary FROM profile
                    WHERE id = $1
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
            WHERE kind IN (SELECT value FROM json_each($3))

            ORDER BY
                CASE
                    -- prioritize results where both the headword and reading match the lemma
                    -- e.g. if you typed あらゆる:
                    -- - the first results would be for the kana あらゆる
                    -- - then the kanji like 汎ゆる
                    WHEN term_record.reading = $2 AND term_record.headword = $2 THEN 0
                    -- then prioritize results where at least the reading or headword are an exact match
                    -- e.g. in 念じる, usually 念ずる comes up first
                    -- but this is obviously a different reading
                    -- so we want to prioritize 念じる
                    WHEN term_record.reading = $2 OR term_record.headword = $2 THEN 1
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
                let term = Term::new(record.headword, record.reading)
                    .context("fetched empty term")?;

                let typed_record = for_kinds!(deserialize_record)
                    .with_context(|| {
                        format!(
                            "failed to deserialize record {term:?} from dictionary {:?} ({source:?})",
                            self.dictionaries().get(&source).map_or("?", |dict| dict.meta.name.as_str())
                        )
                    })?;

                Ok(RecordLookup {
                    bytes_scanned: lemma.len(),
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
        profile_id: ProfileId,
        sentence: &'a str,
        cursor: usize,
        record_kinds: &[impl Borrow<RecordKind> + Send + Sync],
    ) -> Result<Vec<RecordLookup>> {
        // TODO: languages with words separated by e.g. spaces need a different strategy
        let (_, query) = sentence
            .split_at_checked(cursor)
            .context("cursor is not on a UTF-8 character boundary")?;

        let mut records = Vec::new();
        let mut seen_record_ids = HashSet::new();

        for deinflection in self.deinflect(query) {
            for result in self
                .lookup_lemma(profile_id, &deinflection.lemma, record_kinds)
                .await?
            {
                if seen_record_ids.insert(result.record_id) {
                    records.push(RecordLookup {
                        bytes_scanned: deinflection.scan_len,
                        ..result
                    });
                }
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
