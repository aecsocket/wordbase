use std::marker::PhantomData;

use anyhow::{Context, Result};
use sqlx::{QueryBuilder, Sqlite, Transaction, query_builder::Separated};
use wordbase_api::{
    DictionaryId, FrequencyValue, NormString, Record, RecordKind, RecordType, Term,
};

use crate::db;

const BIND_LIMIT: usize = 32766;

pub struct Insert<T> {
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
        _ = sqlx::query!(
            "INSERT INTO record (source, headword, reading, kind, data)
            VALUES ($1, $2, $3, $4, $5)",
            DictionaryId(0).0,
            "",
            "",
            RecordKind::YomitanGlossary as u32,
            &[0u8] as &[u8],
        );
        Self {
            qb: QueryBuilder::new(
                "INSERT INTO record (source, headword, reading, kind, data)
                VALUES ",
            ),
            binds: 0,
            _phantom: PhantomData,
        }
    }

    pub async fn insert<R: RecordType>(
        &mut self,
        tx: &mut Transaction<'_, Sqlite>,
        source: DictionaryId,
        term: Term,
        record: &R,
    ) -> Result<()> {
        let mut scratch = Vec::new();
        db::serialize(&record, &mut scratch).context("failed to serialize record")?;
        let (headword, reading) = term.into_parts();
        self.do_insert::<5>(tx, |mut qb| {
            qb.push_bind(source.0);
            qb.push_bind(headword.map(NormString::into_inner));
            qb.push_bind(reading.map(NormString::into_inner));
            qb.push_bind(R::KIND as u32);
            qb.push_bind(scratch);
        })
        .await
    }
}

impl Insert<FrequencyValue> {
    pub fn new() -> Self {
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

        for _ in 0..ITEMS {
            records
                .insert(
                    &mut tx,
                    source,
                    Term::from_headword("foo").unwrap(),
                    &dict::yomitan::Frequency {
                        display: None,
                        value: None,
                    },
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
}
