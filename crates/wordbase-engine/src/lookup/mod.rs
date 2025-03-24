use {
    crate::{CHANNEL_BUF_CAP, db},
    anyhow::{Context, Result},
    futures::{Stream, StreamExt, never::Never},
    sqlx::{Pool, Sqlite},
    tokio::sync::mpsc,
    tokio_stream::wrappers::ReceiverStream,
    tracing::debug,
    wordbase::protocol::{LookupRequest, LookupResponse},
};

#[derive(Debug, Clone)]
pub struct Lookups {
    db: Pool<Sqlite>,
}

impl Lookups {
    pub(super) fn new(db: Pool<Sqlite>) -> Self {
        Self { db }
    }

    pub async fn lookup(
        &self,
        request: LookupRequest,
    ) -> impl Stream<Item = Result<LookupResponse>> {
        let lemma = &request.text;
        let mut responses = db::term::lookup(lemma, &request.record_kinds);
        responses.fetch(&self.db)
    }
}
