//! Database batch insertion tools.
//!
//! Running individual `INSERT` statements is slow because we have to round-trip
//! the database. So we try to batch as many `VALUES` as possible into each
//! `INSERT` statement - that's what [`Insert`] provides. When we have inserted
//! as many values as we can possibly fit into a single statement (once we hit
//! [`BIND_LIMIT`] number of bindings), we flush the statement and start a new
//! one.
//!
//! **Issue: we can't use `lastInsertRowId` to fetch the ID of the record we just
//! inserted.**
//!
//! To get around this, we first fetch `MAX(id)` of the table, then
//! use this ID + 1 as the next insertion ID. Each time we add a new value to
//! the `INSERT` statement, we increment the ID. This means we don't round-trip
//! the database for each ID we need.
//!
//! **Issue: we may flush inserts into a dependent table before we flush inserts
//! into the dependency table.** For example, `term_record` relies on foreign keys
//! into `record`, but we may flush `term_record` inserts before `record`
//! inserts, meaning we break the foreign key constraint.
//!
//! We can't fix this by just disabling `PRAGMA foreign_keys`, because that
//! breaks `ON DELETE CASCADE`. Instead, we use `DEFERRABLE INITIALLY DEFERRED`
//! to make the constraints only be checked once we `COMMIT`.

use std::marker::PhantomData;

use anyhow::{Context, Result};
use sqlx::{QueryBuilder, Sqlite, Transaction, query_builder::Separated};
use wordbase_api::{
    DictionaryId, FrequencyValue, NormString, Record, RecordId, RecordKind, RecordType, Term,
};

use crate::db;

/// SQLite bind parameter count limit.
const BIND_LIMIT: usize = 32766;

pub struct Inserter<'tx, 'c> {
    pub tx: &'tx mut Transaction<'c, Sqlite>,
    source: DictionaryId,
    last_record_id: i64,
    records: Insert<Record>,
    term_records: Insert<Term>,
    frequencies: Insert<FrequencyValue>,
}

impl<'tx, 'c> Inserter<'tx, 'c> {
    pub async fn new(tx: &'tx mut Transaction<'c, Sqlite>, source: DictionaryId) -> Result<Self> {
        let last_record_id = sqlx::query_scalar!("SELECT MAX(id) FROM record")
            .fetch_one(&mut **tx)
            .await
            .context("failed to fetch last record id")?
            .unwrap_or(0);

        Ok(Self {
            tx,
            source,
            last_record_id,
            records: Insert::<Record>::new(),
            term_records: Insert::<Term>::new(),
            frequencies: Insert::<FrequencyValue>::new(),
        })
    }

    pub async fn flush(&mut self) -> Result<()> {
        self.records
            .flush(self.tx)
            .await
            .context("failed to flush records")?;
        self.term_records
            .flush(self.tx)
            .await
            .context("failed to flush term records")?;
        self.frequencies
            .flush(self.tx)
            .await
            .context("failed to flush frequencies")?;
        Ok(())
    }

    pub async fn record<R: RecordType>(&mut self, record: &R) -> Result<RecordId> {
        let record_id = self.last_record_id.wrapping_add(1);
        self.last_record_id = record_id;
        let record_id = RecordId(record_id);
        self.records
            .insert(self.tx, record_id, self.source, record)
            .await?;
        Ok(record_id)
    }

    pub async fn term_record(&mut self, term: Term, record_id: RecordId) -> Result<()> {
        self.term_records
            .insert(self.tx, self.source, term, record_id)
            .await
    }

    pub async fn frequency(&mut self, term: Term, frequency: FrequencyValue) -> Result<()> {
        self.frequencies
            .insert(self.tx, self.source, term, frequency)
            .await
    }
}

struct Insert<T> {
    qb: QueryBuilder<'static, Sqlite>,
    binds: usize,
    _phantom: PhantomData<T>,
}

impl<T> Insert<T> {
    pub async fn flush(&mut self, tx: &mut Transaction<'_, Sqlite>) -> Result<()> {
        if self.binds == 0 {
            return Ok(());
        }

        self.qb
            .build()
            .execute(&mut **tx)
            .await
            .context("failed to insert")?;
        self.qb.reset();
        self.binds = 0;
        Ok(())
    }

    async fn do_insert<const N: usize>(
        &mut self,
        tx: &mut Transaction<'_, Sqlite>,
        f: impl FnOnce(Separated<'_, '_, Sqlite, &str>),
    ) -> Result<()> {
        if self.binds + N >= BIND_LIMIT {
            self.flush(tx).await.context(
                "failed to flush (error may be related to a previous insert, not the current one)",
            )?;
        }
        if self.binds > 0 {
            self.qb.push(", ");
        }
        self.binds += N;
        self.qb.push("(");

        f(self.qb.separated(", "));

        self.qb.push(")");
        Ok(())
    }
}

impl Insert<Record> {
    pub fn new() -> Self {
        // compile-time guard to make sure the query is valid
        _ = sqlx::query!(
            "INSERT INTO record (id, source, kind, data)
            VALUES ($1, $2, $3, $4)",
            RecordId(0).0,
            DictionaryId(0).0,
            RecordKind::YomitanGlossary as u32,
            &[0u8] as &[u8],
        );
        Self {
            qb: QueryBuilder::new(
                "INSERT INTO record (id, source, kind, data)
                VALUES ",
            ),
            binds: 0,
            _phantom: PhantomData,
        }
    }

    pub async fn insert<R: RecordType>(
        &mut self,
        tx: &mut Transaction<'_, Sqlite>,
        id: RecordId,
        source: DictionaryId,
        record: &R,
    ) -> Result<RecordId> {
        let mut scratch = Vec::new();
        db::serialize(&record, &mut scratch).context("failed to serialize record")?;
        self.do_insert::<4>(tx, |mut qb| {
            qb.push_bind(id.0);
            qb.push_bind(source.0);
            qb.push_bind(R::KIND as u32);
            qb.push_bind(scratch);
        })
        .await?;
        Ok(id)
    }
}

impl Insert<Term> {
    pub fn new() -> Self {
        // compile-time guard to make sure the query is valid
        // we're allowed to `OR IGNORE` because that just means
        // we've inserted a duplicate row
        _ = sqlx::query!(
            "INSERT OR IGNORE INTO term_record (source, headword, reading, record)
            VALUES ($1, $2, $3, $4)",
            DictionaryId(0).0,
            "",
            "",
            RecordId(0).0
        );
        Self {
            qb: QueryBuilder::new(
                "INSERT OR IGNORE INTO term_record (source, headword, reading, record)
                VALUES ",
            ),
            binds: 0,
            _phantom: PhantomData,
        }
    }

    pub async fn insert(
        &mut self,
        tx: &mut Transaction<'_, Sqlite>,
        source: DictionaryId,
        term: Term,
        record_id: RecordId,
    ) -> Result<()> {
        let (headword, reading) = term.into_parts();
        self.do_insert::<4>(tx, |mut qb| {
            qb.push_bind(source.0);
            qb.push_bind(headword.map(NormString::into_inner));
            qb.push_bind(reading.map(NormString::into_inner));
            qb.push_bind(record_id.0);
        })
        .await
    }
}

impl Insert<FrequencyValue> {
    pub fn new() -> Self {
        // compile-time guard to make sure the query is valid
        _ = sqlx::query!(
            "INSERT OR IGNORE INTO frequency (source, headword, reading, mode, value)
            VALUES ($1, $2, $3, $4, $5)",
            DictionaryId(0).0,
            "",
            "",
            0,
            0,
        );
        Self {
            qb: QueryBuilder::new(
                "INSERT OR IGNORE INTO frequency (source, headword, reading, mode, value)
                VALUES ",
            ),
            binds: 0,
            _phantom: PhantomData,
        }
    }

    pub async fn insert(
        &mut self,
        tx: &mut Transaction<'_, Sqlite>,
        source: DictionaryId,
        term: Term,
        frequency: FrequencyValue,
    ) -> Result<()> {
        let (mode, value) = match frequency {
            FrequencyValue::Rank(n) => (0, n),
            FrequencyValue::Occurrence(n) => (1, n),
        };
        let (headword, reading) = term.into_parts();
        self.do_insert::<5>(tx, |mut qb| {
            qb.push_bind(source.0);
            qb.push_bind(headword.map(NormString::into_inner));
            qb.push_bind(reading.map(NormString::into_inner));
            qb.push_bind(mode);
            qb.push_bind(value);
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use sqlx::{Pool, query_scalar};
    use wordbase_api::{DictionaryKind, DictionaryMeta, dict};

    use crate::import::insert_dictionary;

    use super::*;

    #[sqlx::test]
    async fn insert_none(db: Pool<Sqlite>) {
        let mut tx = db.begin().await.unwrap();
        let mut records = Insert::<Record>::new();
        records.flush(&mut tx).await.unwrap();
        tx.commit().await.unwrap();
    }

    #[sqlx::test]
    async fn batch_inserts(db: Pool<Sqlite>) {
        const ITEMS: i64 = 100_000;

        let mut tx = db.begin().await.unwrap();
        let source = insert_dictionary(
            &mut tx,
            &DictionaryMeta::new(DictionaryKind::Yomitan, "dict"),
        )
        .await
        .unwrap();

        let mut records = Insert::<Record>::new();

        for i in 0..ITEMS {
            records
                .insert(
                    &mut tx,
                    RecordId(i),
                    source,
                    &dict::yomitan::Frequency::default(),
                )
                .await
                .unwrap();
        }
        records.flush(&mut tx).await.unwrap();

        tx.commit().await.unwrap();

        assert_eq!(
            ITEMS,
            query_scalar!("SELECT COUNT(*) FROM record")
                .fetch_one(&db)
                .await
                .unwrap()
        );
    }

    #[sqlx::test]
    fn deferred_foreign_keys(db: Pool<Sqlite>) {
        let mut tx = db.begin().await.unwrap();
        let source = insert_dictionary(
            &mut tx,
            &DictionaryMeta::new(DictionaryKind::Yomitan, "dict"),
        )
        .await
        .unwrap();

        let mut insert = Inserter::new(&mut tx, source).await.unwrap();

        let record_id = insert
            .record(&dict::yomitan::Frequency::default())
            .await
            .unwrap();
        insert
            .term_record(Term::from_headword("foo").unwrap(), record_id)
            .await
            .unwrap();

        // make sure that if we flush `term_record` before `record`,
        // that it won't cause foreign key constraint errors
        // (at least until the end of the txn)
        // this is why `term_record.record` is `DEFERRABLE INITIALLY DEFERRED`
        insert.term_records.flush(insert.tx).await.unwrap();
        insert.records.flush(insert.tx).await.unwrap();

        tx.commit().await.unwrap();

        assert_eq!(
            1,
            query_scalar!("SELECT COUNT(*) FROM record")
                .fetch_one(&db)
                .await
                .unwrap()
        );
        assert_eq!(
            1,
            query_scalar!("SELECT COUNT(*) FROM term_record")
                .fetch_one(&db)
                .await
                .unwrap()
        );
    }
}
