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

        // skip the BOS (beginning of sentence) node
        let mut nodes = lattice.bos_node().next()?.iter_next();

        let first_node = nodes.next()?;
        let feature = FeatureFields::new(&first_node.feature)?;
        let lemma = feature.lemma.to_owned();

        let conjugated_len = first_node.length
            + nodes
                .take_while(|node| {
                    let Some(feature) = FeatureFields::new(&node.feature) else {
                        return false;
                    };
                    let stat = i32::from(node.stat);
                    let meta_node = stat == MECAB_BOS_NODE || stat == MECAB_EOS_NODE;
                    !meta_node && !feature.start_of_word()
                })
                .map(|node| node.length)
                .sum::<u16>();

        Some(MecabResponse { lemma })
    })
}

struct FeatureFields<'a> {
    part_of_speech: &'a str,
    subclass1: &'a str,
    _subclass2: &'a str,
    _subclass3: &'a str,
    _conjugation_form: &'a str,
    _conjugation_type: &'a str,
    _reading: &'a str,
    lemma: &'a str,
}

impl<'a> FeatureFields<'a> {
    fn new(text: &'a str) -> Option<Self> {
        info!("{text}");

        let mut parts = text.split(',');
        Some(Self {
            part_of_speech: parts.next()?,
            subclass1: parts.next()?,
            _subclass2: parts.next()?,
            _subclass3: parts.next()?,
            _conjugation_form: parts.next()?,
            _conjugation_type: parts.next()?,
            _reading: parts.next()?,
            lemma: parts.next()?,
        })
    }

    #[must_use]
    fn start_of_word(&self) -> bool {
        match self.part_of_speech {
            // particle
            "助詞" => false,
            // verb
            "動詞" => {
                // 食べ    subclass1: 一般
                // なかっ  subclass1: (empty)
                // た      subclass1: (empty)
                !self.subclass1.is_empty()
            }
            _ => true,
        }
    }
}
