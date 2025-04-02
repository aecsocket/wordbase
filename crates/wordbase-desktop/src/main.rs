#![doc = include_str!("../README.md")]
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]

// mod popup;
mod render;
mod theme;

use {
    anyhow::{Context, Result},
    directories::ProjectDirs,
    foldhash::HashMap,
    relm4::{
        adw::{self, prelude::*},
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    render::{RecordRender, RecordRenderConfig, RecordRenderMsg, RecordRenderResponse},
    std::sync::Arc,
    theme::DefaultTheme,
    tokio::fs,
    tracing::{info, level_filters::LevelFilter, warn},
    tracing_subscriber::EnvFilter,
    wordbase::{Dictionary, DictionaryId, RecordKind},
    wordbase_engine::Engine,
};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();

    RelmApp::new("io.github.aecsocket.Wordbase").run_async::<App>(());
}

#[derive(Debug)]
struct App {
    engine: Engine,
    dictionaries: Arc<HashMap<DictionaryId, Dictionary>>,
    _default_theme_watcher: Option<notify::RecommendedWatcher>,
    renderer: Controller<RecordRender>,
}

#[derive(Debug)]
enum AppMsg {
    Lookup { query: String },
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Window {
            adw::ToolbarView {
                set_top_bar_style: adw::ToolbarStyle::Raised,
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    set_title_widget = &gtk::SearchEntry {
                        set_hexpand: true,
                        connect_search_changed => move |widget| {
                            sender.input(AppMsg::Lookup { query: widget.text().into() });
                        },
                    },
                },

                model.renderer.widget(),
            },
        }
    }

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local]
            root {
                set_title: Some("Wordbase"),
                set_default_width: 480,
                set_default_height: 600,

                #[name(spinner)]
                adw::Spinner {}
            }
        }
        Some(LoadingWidgets::new(root, spinner))
    }

    async fn init(
        (): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let init = init().await.unwrap();
        let renderer = RecordRender::builder()
            .launch(RecordRenderConfig {
                default_theme: init.default_theme.theme,
                custom_theme: None,
            })
            .forward(sender.input_sender(), |response| match response {
                RecordRenderResponse::RequestLookup { query } => AppMsg::Lookup { query },
            });

        let renderer_sender = renderer.sender().clone();
        let default_theme_watcher = init
            .default_theme
            .watcher_factory
            .create(move |theme| {
                info!("Default theme changed");
                _ = renderer_sender.send(RecordRenderMsg::SetDefaultTheme(Arc::new(theme)));
            })
            .unwrap();

        let model = Self {
            engine: init.engine,
            dictionaries: init.dictionaries,
            _default_theme_watcher: default_theme_watcher,
            renderer,
        };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppMsg::Lookup { query } => {
                let records = match self.engine.lookup_lemma(&query, RecordKind::ALL).await {
                    Ok(records) => records,
                    Err(err) => {
                        warn!("Failed to lookup records for {query:?}: {err:?}");
                        return;
                    }
                };
                _ = self.renderer.sender().send(RecordRenderMsg::Lookup {
                    dictionaries: self.dictionaries.clone(),
                    records,
                });
            }
        }
    }
}

#[derive(Debug)]
struct AppInit {
    engine: Engine,
    dictionaries: Arc<HashMap<DictionaryId, Dictionary>>,
    default_theme: DefaultTheme,
}

async fn init() -> Result<AppInit> {
    let dirs = ProjectDirs::from("io.github", "aecsocket", "Wordbase")
        .context("failed to get default app directories")?;
    let data_path = dirs.data_dir();
    info!("Using {data_path:?} as data path");
    fs::create_dir_all(data_path)
        .await
        .context("failed to create data directory")?;

    let db_path = data_path.join("wordbase.db");
    let (engine, engine_task) = Engine::new(db_path).await?;
    tokio::spawn(async move {
        engine_task.await.expect("engine error");
    });
    let dictionaries = engine
        .dictionaries()
        .await
        .context("failed to fetch initial dictionaries")?
        .into_iter()
        .map(|dict| (dict.id, dict))
        .collect::<HashMap<_, _>>();

    let default_theme = theme::default_theme()
        .await
        .context("failed to get default theme")?;
    Ok(AppInit {
        engine,
        dictionaries: Arc::new(dictionaries),
        default_theme,
    })
}
