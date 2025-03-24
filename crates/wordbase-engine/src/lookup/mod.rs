use anyhow::{Context, Result};
use futures::{SinkExt, Stream, StreamExt, channel::mpsc, never::Never};
use sqlx::{Pool, Sqlite};
use wordbase::protocol::{LookupRequest, LookupResponse};

use crate::{CHANNEL_BUF_CAP, db};

#[derive(Debug, Clone)]
pub struct Lookups {
    send_request: mpsc::Sender<Request>,
}

pub fn run(db: Pool<Sqlite>) -> (Lookups, impl Future<Output = Result<Never>>) {
    let (send_request, recv_request) = mpsc::channel(CHANNEL_BUF_CAP);
    (Lookups { send_request }, backend(db, recv_request))
}

#[derive(Debug)]
enum Request {
    Lookup {
        request: LookupRequest,
        send_response: mpsc::Sender<Result<LookupResponse>>,
    },
}

impl Lookups {
    pub async fn lookup(
        &mut self,
        request: LookupRequest,
    ) -> Result<impl Stream<Item = Result<LookupResponse>>> {
        let (send_response, recv_response) = mpsc::channel(CHANNEL_BUF_CAP);
        self.send_request
            .send(Request::Lookup {
                request,
                send_response,
            })
            .await?;
        Ok(recv_response)
    }
}

async fn backend(db: Pool<Sqlite>, mut recv_request: mpsc::Receiver<Request>) -> Result<Never> {
    loop {
        let request = recv_request
            .next()
            .await
            .context("request channel closed")?;

        match request {
            Request::Lookup {
                request,
                send_response,
            } => {
                let lemma = &request.text;
                db::term::lookup(lemma, &request.record_kinds)
                    .fetch(&db)
                    .forward(send_response);
            }
        }
    }
}
