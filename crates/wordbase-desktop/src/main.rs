#![doc = include_str!("../README.md")]
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]

// mod popup;
mod render;

use {
    anyhow::{Context, Result},
    directories::ProjectDirs,
    relm4::{
        adw::{self, prelude::*},
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    tokio::fs,
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    webkit6::prelude::WebViewExt,
    wordbase::RecordKind,
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
    web_view: webkit6::WebView,
}

#[derive(Debug)]
enum AppMsg {
    Search { query: String },
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Window {
            gtk::Box {
                set_margin_top: 16,
                set_margin_bottom: 16,
                set_margin_start: 16,
                set_margin_end: 16,
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 16,

                gtk::SearchEntry {
                    set_valign: gtk::Align::Start,
                    connect_search_changed => move |widget| {
                        sender.input(AppMsg::Search { query: widget.text().into() });
                    },
                },

                #[name(web_view)]
                webkit6::WebView {
                    set_hexpand: true,
                    set_vexpand: true,
                }
            }
        }
    }

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local]
            root {
                set_title: Some("Wordbase"),

                #[name(spinner)]
                adw::Spinner {}
            }
        }
        Some(LoadingWidgets::new(root, spinner))
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let engine = create_engine().await.expect("failed to create engine");
        let widgets = view_output!();
        let model = Self {
            engine,
            web_view: widgets.web_view.clone(),
        };
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppMsg::Search { query } => {
                println!("query");

                let Ok(records) = self.engine.lookup_lemma(&query, RecordKind::ALL).await else {
                    return;
                };

                let html = render::to_html(records);
                println!("{}", html.0);

                self.web_view.load_html(&html.0, None);
            }
        }
    }
}

async fn create_engine() -> Result<Engine> {
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
    Ok(engine)
}
