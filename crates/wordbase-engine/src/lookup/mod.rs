use {
    crate::db,
    anyhow::Result,
    futures::Stream,
    sqlx::{Pool, Sqlite},
    wordbase::protocol::{LookupRequest, LookupResponse},
};

#[derive(Debug, Clone)]
pub struct Lookups {
    db: Pool<Sqlite>,
}

impl Lookups {
    pub(super) const fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn lookup(
        &self,
        request: LookupRequest,
    ) -> impl Stream<Item = Result<LookupResponse>> {
        let lemma = &request.text;
        db::term::lookup(&self.db, lemma, &request.record_kinds).await
    }
}
