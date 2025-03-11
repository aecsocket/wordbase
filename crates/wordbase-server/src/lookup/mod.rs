mod mecab;

use {
    crate::{CHANNEL_BUF_CAP, Config, db},
    anyhow::{Context, Result, bail},
    futures::never::Never,
    sqlx::{Pool, Sqlite},
    std::sync::Arc,
    tokio::sync::{mpsc, oneshot},
    wordbase::protocol::{LookupRequest, LookupResponse},
};

#[derive(Debug, Clone)]
pub struct Client {
    send_request: mpsc::Sender<Request>,
}

impl Client {
    pub fn new(
        config: Arc<Config>,
        db: Pool<Sqlite>,
    ) -> (Self, impl Future<Output = Result<Never>>) {
        let (send_request, recv_request) = mpsc::channel(CHANNEL_BUF_CAP);
        (Self { send_request }, run(config, db, recv_request))
    }

    pub async fn lookup(&self, request: LookupRequest) -> Result<Vec<LookupResponse>> {
        let (send_response, recv_response) = oneshot::channel();
        self.send_request
            .send(Request {
                request,
                send_response,
            })
            .await?;
        let result = recv_response.await?;
        result
    }
}

#[derive(Debug)]
struct Request {
    request: LookupRequest,
    send_response: oneshot::Sender<Result<Vec<LookupResponse>>>,
}

async fn run(
    config: Arc<Config>,
    db: Pool<Sqlite>,
    mut recv_request: mpsc::Receiver<Request>,
) -> Result<Never> {
    let mecab = mecab::Client::new().await;

    loop {
        let request = recv_request
            .recv()
            .await
            .context("lookup request channel closed")?;

        let result = handle_request(&config, &db, &mecab, request.request).await;
        _ = request.send_response.send(result);
    }
}

async fn handle_request(
    config: &Config,
    db: &Pool<Sqlite>,
    mecab: &mecab::Client,
    request: LookupRequest,
) -> Result<Vec<LookupResponse>> {
    let LookupRequest { text, record_kinds } = request;

    // count like this instead of using `.count()`
    // because `count` does not short-circuit
    let mut request_chars = text.chars();
    let mut num_request_chars = 0u64;
    let max_request_len = config.lookup.max_request_len;
    while let Some(_) = request_chars.next() {
        num_request_chars += 1;
        if num_request_chars > max_request_len {
            bail!("request too long - max {max_request_len} characters");
        }
    }

    let Some(mecab) = mecab.get_info(text).await? else {
        return Ok(Vec::new());
    };

    let records = db::term::lookup(db, &mecab.lemma, &record_kinds)
        .await
        .context("failed to fetch records")?;
    Ok(records)
}
