use {
    crate::{Engine, db},
    anyhow::{Context, Result, bail},
    futures::{StreamExt, TryStreamExt},
    std::borrow::Borrow,
    wordbase::{DictionaryId, FrequencyValue, Record, RecordKind, RecordLookup, Term, for_kinds},
};

impl Engine {
    #[expect(clippy::missing_panics_doc, reason = "shouldn't panic")]
    pub async fn lookup_lemma(
        &self,
        lemma: &str,
        record_kinds: impl IntoIterator<Item = impl Borrow<RecordKind>>,
    ) -> Result<Vec<RecordLookup>> {
        let record_kinds = record_kinds
            .into_iter()
            .map(|kind| format!("{}", *kind.borrow() as u16))
            .collect::<Vec<_>>();
        let kinds_str = serde_json::to_string(&record_kinds)
            .expect("should be able to generate JSON for record kinds array");

        let query = sqlx::query!(
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
            INNER JOIN profile_enabled_dictionary ped ON (ped.profile = config.current_profile AND \
             ped.dictionary = dictionary.id)

            -- find which terms reference this record, either through the headword or reading
            INNER JOIN term_record ON (
                term_record.record = record.id
                AND (term_record.headword = $1 OR term_record.reading = $1)
            )

            -- join on frequency information, for the `ORDER BY` below
            LEFT JOIN frequency ON (
                -- only use frequency info from the currently selected sorting dict in this profile
                frequency.source = (SELECT sorting_dictionary FROM profile WHERE id = \
             config.current_profile)
                AND (frequency.headword = term_record.headword AND frequency.reading = \
             term_record.reading)
            )

            -- only include records for the given record kinds
            WHERE kind IN (SELECT value FROM json_each($2))

            ORDER BY
                -- user-specified dictionary sorting position always takes priority
                dictionary.position,
                CASE
                    -- put entries without an explicit frequency value first
                    WHEN frequency.mode IS NULL THEN 0
                    ELSE 1
                END,
                CASE
                    -- frequency rank
                    WHEN frequency.mode = 0 THEN  frequency.value
                    -- frequency occurrence
                    WHEN frequency.mode = 1 THEN -frequency.value
                    ELSE 0
                END",
            lemma,
            kinds_str
        );

        query.fetch(&self.db).map(|record| {
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
                source: DictionaryId(record.source),
                term: Term::new(record.headword, record.reading).context("fetched empty term")?,
                record: for_kinds!(deserialize_record),
                frequency: match (record.frequency_mode, record.frequency_value.map(u64::try_from)) {
                    (Some(0), Some(Ok(value))) => Some(FrequencyValue::Rank(value)),
                    (Some(1), Some(Ok(value))) => Some(FrequencyValue::Occurrence(value)),
                    _ => None
                }
            })
        })
        .try_collect()
        .await
    }
}
