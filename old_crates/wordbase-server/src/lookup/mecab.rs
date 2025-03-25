use {
    crate::CHANNEL_BUF_CAP,
    anyhow::{Context, Result},
    futures::never::Never,
    mecab::{Model, Tagger},
    std::{cell::OnceCell, sync::LazyLock},
    tokio::sync::{mpsc, oneshot},
    tracing::info,
};

#[derive(Debug, Clone)]
pub struct Client {
    send_request: mpsc::Sender<Request>,
}

impl Client {
    pub async fn new() -> Self {
        let (send_request, recv_request) = mpsc::channel(CHANNEL_BUF_CAP);
        tokio::spawn(run(recv_request));
        Self { send_request }
    }

    pub async fn get_info(&self, text: impl Into<String>) -> Result<Option<MecabInfo>> {
        let (send_info, recv_info) = oneshot::channel();
        self.send_request
            .send(Request {
                text: text.into(),
                send_info,
            })
            .await?;
        let info = recv_info.await?;
        Ok(info)
    }
}

#[derive(Debug)]
pub struct MecabInfo {
    pub lemma: String,
}

#[derive(Debug)]
struct Request {
    text: String,
    send_info: oneshot::Sender<Option<MecabInfo>>,
}

static MODEL: LazyLock<Model> = LazyLock::new(|| {
    let model = Model::new("-Ochamame");

    for dictionary in model.dictionary_info().iter() {
        info!(
            "Loaded MeCab dictionary {:?} with {} entries",
            dictionary.filename, dictionary.size
        );
    }

    model
});

thread_local! {
    static TAGGER: OnceCell<Tagger> = const { OnceCell::new() };
}

async fn run(mut recv_request: mpsc::Receiver<Request>) -> Result<Never> {
    // initialize the model before we even get any requests
    _ = *MODEL;

    loop {
        let request = recv_request
            .recv()
            .await
            .context("request channel closed")?;
        let info = compute_info(request.text);
        _ = request.send_info.send(info);
    }
}

fn compute_info(text: String) -> Option<MecabInfo> {
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

        Some(MecabInfo { lemma })
    })
}

struct FeatureFields<'a> {
    _part_of_speech: &'a str,
    _subclass1: &'a str,
    _subclass2: &'a str,
    _subclass3: &'a str,
    _conjugation_form: &'a str,
    _conjugation_type: &'a str,
    _reading: &'a str,
    lemma: &'a str,
}

impl<'a> FeatureFields<'a> {
    fn new(text: &'a str) -> Option<Self> {
        let mut parts = text.split(',');
        Some(Self {
            _part_of_speech: parts.next()?,
            _subclass1: parts.next()?,
            _subclass2: parts.next()?,
            _subclass3: parts.next()?,
            _conjugation_form: parts.next()?,
            _conjugation_type: parts.next()?,
            _reading: parts.next()?,
            lemma: parts.next()?,
        })
    }
}
