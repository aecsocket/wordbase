#![doc = include_str!("../README.md")]
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]

mod overlay;
mod platform;
mod popup;
mod record;
mod theme;

use {
    anyhow::{Context, Result},
    directories::ProjectDirs,
    futures::never::Never,
    platform::Platform,
    record::view::{RecordView, RecordViewMsg},
    relm4::{
        adw::{self, prelude::*},
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    std::sync::Arc,
    tokio::{fs, sync::mpsc},
    tracing::{info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::Lookup,
    wordbase_engine::{Engine, texthook::TexthookerEvent},
};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .init();
    glib::log_set_default_handler(glib::rust_log_handler);

    let app = adw::Application::builder()
        .application_id("io.github.aecsocket.Wordbase")
        .build();
    RelmApp::from_app(app.clone()).run_async::<App>(app);
}

#[derive(Debug)]
struct App {
    engine: Engine,
    record_view: AsyncController<RecordView>,
}

#[derive(Debug)]
enum AppMsg {
    Lookup { query: String },
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = adw::Application;
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

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

    view! {
        adw::Window {
            adw::ToolbarView {
                set_top_bar_style: adw::ToolbarStyle::Raised,
                add_top_bar = &adw::HeaderBar {
                    #[wrap(Some)]
                    #[name(search_entry)]
                    set_title_widget = &gtk::SearchEntry {
                        set_hexpand: true,
                        connect_search_changed => move |widget| {
                            sender.input(AppMsg::Lookup { query: widget.text().into() });
                        },
                    },
                },

                model.record_view.widget(),
            },
        }
    }

    async fn init(
        app: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let init = init(app).await.unwrap();
        let record_view = RecordView::builder().launch(init.engine.clone()).detach();

        let model = Self {
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
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppMsg::Lookup { query } => {
                _ = self
                    .record_view
                    .sender()
                    .send(RecordViewMsg::Lookup(Lookup {
                        context: query,
                        cursor: 0,
                    }));
            }
        }
    }
}

#[derive(Debug)]
struct AppInit {
    engine: Engine,
}

async fn init(app: adw::Application) -> Result<AppInit> {
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

    // popup
    let (send_popup_request, recv_popup_request) = mpsc::channel(CHANNEL_BUF_CAP);
    glib::spawn_future_local(popup::run(
        engine.clone(),
        app.clone(),
        platform.clone(),
        recv_popup_request,
    ));

    // overlay
    let (send_sentence, recv_sentence) = mpsc::channel(CHANNEL_BUF_CAP);
    let (texthooker_task, mut recv_texthooker_event) = engine
        .texthooker_task()
        .await
        .context("failed to start texthooker task")?;
    tokio::spawn(texthooker_task);
    glib::spawn_future_local(overlay::run(
        app,
        platform,
        recv_sentence,
        send_popup_request,
    ));
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

    Ok(AppInit { engine })
}

const CHANNEL_BUF_CAP: usize = 4;
