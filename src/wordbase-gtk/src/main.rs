#![doc = include_str!("../README.md")]
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]
#![allow(clippy::wildcard_imports, reason = "used for `imp` modules")]
#![allow(
    clippy::new_without_default,
    reason = "`gtk` doesn't follow this convention"
)]

mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

use {
    adw::prelude::*,
    anyhow::{Context, Result, anyhow},
    arc_swap::ArcSwap,
    derive_more::Debug,
    glib::clone,
    relm4::{MessageBroker, RelmApp, loading_widgets::LoadingWidgets, prelude::*, view},
    std::{
        path::PathBuf,
        sync::{Arc, LazyLock, OnceLock},
    },
    tokio::sync::{OnceCell, broadcast},
    tracing::{error, info, level_filters::LevelFilter, warn},
    tracing_subscriber::EnvFilter,
    wordbase::{
        Profile, ProfileId, RecordEntry, Wordbase, dictionary::Dictionaries, lookup::Lookups,
        render::Renderer,
    },
};

const APP_ID: &str = "app.wordbase.Wordbase";

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    glib::log_set_default_handler(glib::rust_log_handler);
    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);

    // let settings = gio::Settings::new(APP_ID);
    let data_dir = wordbase_desktop::data_dir().context("failed to get default data directory")?;

    RelmApp::new(APP_ID).run_async::<App>(App::new(data_dir));
    Ok(())
}

#[derive(Debug)]
pub struct App {
    data_dir: PathBuf,
    engine: OnceCell<Wordbase>,
    lookups: OnceCell<Lookups>,
    renderer: OnceCell<Renderer>,
}

impl App {
    pub fn new(data_dir: impl Into<PathBuf>) -> Self {
        Self {
            data_dir: data_dir.into(),
            engine: OnceCell::new(),
            lookups: OnceCell::new(),
            renderer: OnceCell::new(),
        }
    }

    // private to prevent users mutating the engine without going through our
    // functions which will trigger events
    async fn engine(&self) -> &Wordbase {
        self.engine
            .get_or_init(|| async {
                Wordbase::new(&self.data_dir)
                    .await
                    .inspect(|_| info!("Initialized engine"))
                    .unwrap()
            })
            .await
    }

    async fn lookups(&self) -> &Lookups {
        self.lookups
            .get_or_init(|| async {
                Lookups::new()
                    .await
                    .inspect(|_| info!("Initialized lookups"))
                    .unwrap()
            })
            .await
    }

    async fn renderer(&self) -> &Renderer {
        self.renderer
            .get_or_init(|| async {
                Renderer::new()
                    .inspect(|_| info!("Initialized renderer"))
                    .unwrap()
            })
            .await
    }

    pub async fn dictionaries(&self) -> Arc<Dictionaries> {
        self.engine().await.dictionaries()
    }

    pub async fn lookup(
        &self,
        profile_id: ProfileId,
        sentence: &str,
        cursor: usize,
    ) -> Result<Vec<RecordEntry>> {
        self.lookups()
            .await
            .lookup(self.engine().await, profile_id, sentence, cursor)
            .await
    }
}

impl AsyncComponent for App {
    type CommandOutput = ();
    type Input = ();
    type Output = ();
    type Init = Self;
    type Root = adw::ApplicationWindow;
    type Widgets = ();

    fn init_root() -> Self::Root {
        adw::ApplicationWindow::builder()
            .application(&relm4::main_adw_application())
            .title("Wordbase")
            .build()
    }

    async fn init(
        app: Self::Init,
        root: Self::Root,
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let dictionaries = app.dictionaries().await;
        let lk = app.lookup(ProfileId(1), "読む", 0).await.unwrap();
        root.set_content(Some(&gtk::Label::new(Some(&format!(
            "dicts = {dictionaries:?} / lk = {lk:?}"
        )))));

        AsyncComponentParts {
            model: app,
            widgets: (),
        }
    }
}
