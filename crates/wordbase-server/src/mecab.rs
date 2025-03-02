use anyhow::{Context, Result};
use futures::never::Never;
use mecab::{MECAB_BOS_NODE, MECAB_EOS_NODE, Model};
use tokio::sync::{mpsc, oneshot};
use tracing::info;

#[derive(Debug)]
pub struct MecabRequest {
    pub text: String,
    pub send_response: oneshot::Sender<MecabResponse>,
}

#[derive(Debug)]
pub struct MecabResponse {
    pub deinflected: Option<String>,
}

// Theoretically, this can be multi-threaded, since `Model` is `Send + Sync`
// and we can make a `Tagger` and `Lattice` per thread.
// TODO possible optimization
#[expect(clippy::future_not_send, reason = "`mecab` types are not `Send`")]
pub async fn run(mut recv_request: mpsc::Receiver<MecabRequest>) -> Result<Never> {
    let model = Model::new("");

    for dictionary in model.dictionary_info().iter() {
        info!(
            "Loaded dictionary {:?} with {} entries",
            dictionary.filename, dictionary.size
        );
    }

    loop {
        let tagger = model.create_tagger();
        let mut lattice = model.create_lattice();

        let request = recv_request
            .recv()
            .await
            .context("request channel closed")?;
        lattice.set_sentence(request.text);
        tagger.parse(&lattice);

        let deinflected = lattice
            .bos_node()
            .iter_next()
            .find(|node| {
                let stat = i32::from(node.stat);
                stat != MECAB_BOS_NODE && stat != MECAB_EOS_NODE
            })
            .and_then(|node| node.feature.split(',').nth(7).map(ToOwned::to_owned));
        _ = request.send_response.send(MecabResponse { deinflected });
    }
}
