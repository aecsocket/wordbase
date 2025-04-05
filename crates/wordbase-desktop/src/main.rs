#![doc = include_str!("../README.md")]
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]
#![allow(clippy::wildcard_imports, reason = "used for `imp` modules")]
#![allow(
    clippy::new_without_default,
    reason = "`gtk` doesn't follow this convention"
)]

mod overlay;
mod platform;
mod popup;
mod record;
mod theme;

mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

use {
    anyhow::{Context, Result},
    directories::ProjectDirs,
    foldhash::HashMap,
    futures::never::Never,
    platform::Platform,
    record::view::{RecordView, RecordViewConfig, RecordViewMsg},
    relm4::{
        adw::{self, gio, prelude::*},
        css::classes,
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    std::sync::Arc,
    tokio::{fs, sync::mpsc},
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{Dictionary, DictionaryId},
    wordbase_engine::{Engine, texthook::TexthookerEvent},
};

const APP_ID: &str = "io.github.aecsocket.Wordbase";

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    glib::log_set_default_handler(glib::rust_log_handler);
    relm4_icons::initialize_icons(icon_names::GRESOURCE_BYTES, icon_names::RESOURCE_PREFIX);

    let app = adw::Application::builder().application_id(APP_ID).build();
    let settings = gio::Settings::new(APP_ID);
    RelmApp::from_app(app.clone()).run_async::<App>(AppConfig { app, settings });
}

type Dictionaries = HashMap<DictionaryId, Dictionary>;

#[derive(Debug)]
struct App {
    app: adw::Application,
    engine: Engine,
    record_view: AsyncController<RecordView>,
}

#[derive(Debug)]
struct AppConfig {
    app: adw::Application,
    settings: gio::Settings,
}

#[derive(Debug)]
enum AppMsg {
    Quit,
    Lookup { query: String },
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = AppConfig;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        view! {
            #[local]
            root {
                set_title: Some("Wordbase"),
                set_hide_on_close: true,

                #[name(spinner)]
                adw::Spinner {}
            }
        }
        Some(LoadingWidgets::new(root, spinner))
    }

    view! {
        adw::Window {
            adw::ToolbarView {
                set_top_bar_style: adw::ToolbarStyle::Raised,
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    #[name(search_entry)]
                    set_title_widget = &gtk::SearchEntry {
                        set_hexpand: true,
                        connect_search_changed[sender] => move |widget| {
                            sender.input(AppMsg::Lookup { query: widget.text().into() });
                        },
                    },

                    pack_start = &gtk::Button {
                        add_css_class: classes::FLAT,
                        set_widget_name: "Quit",
                        set_icon_name: "window-close-symbolic",
                        connect_clicked[sender] => move |_| {
                            sender.input(AppMsg::Quit);
                        },
                    },
                },

                model.record_view.widget(),
            },
        }
    }

    async fn init(
        init: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let app = init.app.clone();
        init.settings
            .bind("manager-width", &root, "default-width")
            .build();
        init.settings
            .bind("manager-height", &root, "default-height")
            .build();

        let init = init_app(init).await.unwrap();
        let record_view = RecordView::builder()
            .launch(RecordViewConfig {
                engine: init.engine.clone(),
                dictionaries: init.dictionaries,
            })
            .detach();

        let model = Self {
            app,
            engine: init.engine,
            record_view,
        };
        let widgets = view_output!();
        widgets.search_entry.grab_focus();
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            AppMsg::Quit => self.app.quit(),
            AppMsg::Lookup { query } => {
                _ = self
                    .record_view
                    .sender()
                    .send(RecordViewMsg::Lookup { query });
            }
        }
    }
}

#[derive(Debug)]
struct AppInit {
    engine: Engine,
    dictionaries: Arc<Dictionaries>,
}

async fn init_app(AppConfig { app, settings }: AppConfig) -> Result<AppInit> {
    let platform = Arc::<dyn Platform>::from(
        platform::default()
            .await
            .context("failed to create platform")?,
    );

    let dirs = ProjectDirs::from("io.github", "aecsocket", "Wordbase")
        .context("failed to get default app directories")?;
    let data_path = dirs.data_dir();
    info!("Using {data_path:?} as data path");
    fs::create_dir_all(data_path)
        .await
        .context("failed to create data directory")?;

    let db_path = data_path.join("wordbase.db");
    let engine = Engine::new(db_path)
        .await
        .context("failed to create engine")?;

    let dictionaries = Arc::<Dictionaries>::new(
        engine
            .dictionaries()
            .await
            .context("failed to fetch initial dictionaries")?
            .into_iter()
            .map(|dict| (dict.id, dict))
            .collect(),
    );

    let mut popup = popup::connector(
        &app,
        &platform,
        RecordViewConfig {
            engine: engine.clone(),
            dictionaries: dictionaries.clone(),
        },
    )
    .await?
    .detach();
    popup.detach_runtime();
    settings
        .bind("popup-width", popup.widget(), "default-width")
        .build();
    settings
        .bind("popup-height", popup.widget(), "default-height")
        .build();

    // overlay
    let (send_sentence, recv_sentence) = mpsc::channel(CHANNEL_BUF_CAP);
    let (texthooker_task, mut recv_texthooker_event) = engine
        .texthooker_task()
        .await
        .context("failed to start texthooker task")?;
    tokio::spawn(texthooker_task);
    glib::spawn_future_local({
        let engine = engine.clone();
        async move {
            overlay::run(
                app,
                platform,
                engine.clone(),
                recv_sentence,
                popup.sender().clone(),
            )
            .await
            .expect("overlay task error")
        }
    });
    // forward pull texthooker events to overlay
    tokio::spawn(async move {
        let _: Option<Never> = async move {
            loop {
                let texthooker_event = recv_texthooker_event.recv().await?;
                if let TexthookerEvent::Sentence(sentence) = texthooker_event {
                    send_sentence.send(sentence).await.ok()?;
                }
            }
        }
        .await;
    });
    // TODO: forward server sentence events to overlay

    Ok(AppInit {
        engine,
        dictionaries,
    })
}

const CHANNEL_BUF_CAP: usize = 4;
