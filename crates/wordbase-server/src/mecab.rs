use std::{cell::OnceCell, sync::LazyLock};

use anyhow::{Context, Result};
use futures::never::Never;
use mecab::{MECAB_BOS_NODE, MECAB_EOS_NODE, Model, Tagger};
use tokio::sync::{mpsc, oneshot};
use tracing::info;

#[derive(Debug)]
pub struct MecabRequest {
    pub text: String,
    pub send_response: oneshot::Sender<Option<MecabResponse>>,
}

#[derive(Debug)]
pub struct MecabResponse {
    pub conjugated_len: u64,
    pub lemma: String,
}

static MODEL: LazyLock<Model> = LazyLock::new(|| {
    let model = Model::new("");

    for dictionary in model.dictionary_info().iter() {
        info!(
            "Loaded dictionary {:?} with {} entries",
            dictionary.filename, dictionary.size
        );
    }

    model
});

thread_local! {
    static TAGGER: OnceCell<Tagger> = const { OnceCell::new() };
}

pub async fn run(mut recv_request: mpsc::Receiver<MecabRequest>) -> Result<Never> {
    _ = *MODEL;

    loop {
        let request = recv_request
            .recv()
            .await
            .context("request channel closed")?;
        let response = respond(request.text);
        _ = request.send_response.send(response);
    }
}

fn respond(text: String) -> Option<MecabResponse> {
    TAGGER.with(|tagger| {
        let tagger = tagger.get_or_init(|| MODEL.create_tagger());
        let mut lattice = MODEL.create_lattice();
        lattice.set_sentence(text);
        tagger.parse(&lattice);

        let node = lattice.bos_node().iter_next().find(|node| {
            let stat = i32::from(node.stat);
            stat != MECAB_BOS_NODE && stat != MECAB_EOS_NODE
        })?;
        let lemma = node.feature.split(',').nth(6).map(ToOwned::to_owned)?;
        Some(MecabResponse {
            conjugated_len: 1,
            lemma,
        })
    })
}
