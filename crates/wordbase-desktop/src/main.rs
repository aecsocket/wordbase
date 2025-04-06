#![doc = include_str!("../README.md")]
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]
#![allow(clippy::wildcard_imports, reason = "used for `imp` modules")]
#![allow(
    clippy::new_without_default,
    reason = "`gtk` doesn't follow this convention"
)]

mod manager;
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
    futures::never::Never,
    manager::Manager,
    platform::Platform,
    popup::{Popup, PopupMsg, PopupResponse},
    record::view::{RecordView, RecordViewMsg, RecordViewResponse},
    relm4::{
        adw::{self, gio, prelude::*},
        css::classes,
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    std::sync::Arc,
    tokio::{fs, sync::mpsc},
    tracing::{error, info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::{Lookup, PopupRequest, ProfileId},
    wordbase_engine::Engine,
};

const APP_ID: &str = "io.github.aecsocket.Wordbase";

const ACTION_PROFILE: &str = "profile";

fn gettext(s: &str) -> &str {
    s
}

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

#[derive(Debug)]
struct App {
    app: adw::Application,
    manager: AsyncController<Manager>,
    engine: Engine,
    record_view: AsyncController<RecordView>,
    popup: AsyncController<Popup>,
    last_popup_send_result: Option<mpsc::Sender<RecordViewResponse>>,
}

#[derive(Debug)]
struct AppConfig {
    app: adw::Application,
    settings: gio::Settings,
}

#[derive(Debug)]
enum AppMsg {
    Quit,
    Present,
    Lookup {
        query: String,
    },
    Popup {
        request: PopupRequest,
        send_result: mpsc::Sender<RecordViewResponse>,
    },
    #[doc(hidden)]
    PopupHidden,
    #[doc(hidden)]
    PopupViewResponse(RecordViewResponse),
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
        config: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let app = config.app.clone();
        let settings = config.settings.clone();
        settings
            .bind("manager-width", &root, "default-width")
            .build();
        settings
            .bind("manager-height", &root, "default-height")
            .build();

        let init = init_app(config, &sender).await.unwrap();
        let manager = Manager::builder()
            .launch((init.engine.clone(), settings.clone()))
            .detach();
        manager.widget().present();
        let record_view = RecordView::builder().launch(init.engine.clone()).detach();

        let model = Self {
            app,
            manager,
            engine: init.engine,
            record_view,
            popup: init.popup,
            last_popup_send_result: None,
        };
        let widgets = view_output!();
        widgets.search_entry.grab_focus();
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppMsg::Quit => {
                self.app.quit();
            }
            AppMsg::Present => {
                root.present();
            }
            AppMsg::Lookup { query } => {
                _ = self
                    .record_view
                    .sender()
                    .send(RecordViewMsg::Lookup(Lookup {
                        context: query,
                        cursor: 0,
                    }));
            }
            AppMsg::Popup {
                request,
                send_result,
            } => {
                _ = self.popup.sender().send(PopupMsg::Request(request));
                self.last_popup_send_result = Some(send_result);
            }
            AppMsg::PopupHidden => {
                self.last_popup_send_result = None;
            }
            AppMsg::PopupViewResponse(resp) => {
                if let Some(send_result) = &self.last_popup_send_result {
                    _ = send_result.send(resp).await;
                }
            }
        }
    }
}

#[derive(Debug)]
struct AppInit {
    engine: Engine,
    popup: AsyncController<Popup>,
}

async fn init_app(
    AppConfig { app, settings }: AppConfig,
    sender: &AsyncComponentSender<App>,
) -> Result<AppInit> {
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

    // actions
    setup_profile_action(&app, engine.clone());

    let popup = popup::connector(&app, &platform, engine.clone())
        .await?
        .forward(sender.input_sender(), |resp| match resp {
            PopupResponse::Hidden => AppMsg::PopupHidden,
            PopupResponse::OpenSettings => AppMsg::Present,
            PopupResponse::View(resp) => AppMsg::PopupViewResponse(resp),
        });
    settings
        .bind("popup-width", popup.widget(), "default-width")
        .build();
    settings
        .bind("popup-height", popup.widget(), "default-height")
        .build();

    // overlay
    // let (send_sentence, recv_sentence) = mpsc::channel(CHANNEL_BUF_CAP);
    // let (texthooker_task, mut recv_texthooker_event) = engine
    //     .texthooker_task()
    //     .await
    //     .context("failed to start texthooker task")?;
    // tokio::spawn(texthooker_task);
    // glib::spawn_future_local({
    //     let engine = engine.clone();
    //     let to_app = sender.input_sender().clone();
    //     async move {
    //         overlay::run(app, platform, engine.clone(), recv_sentence, to_app)
    //             .await
    //             .expect("overlay task error")
    //     }
    // });
    // // forward pull texthooker events to overlay
    // tokio::spawn(async move {
    //     let _: Option<Never> = async move {
    //         loop {
    //             let texthooker_event = recv_texthooker_event.recv().await?;
    //             if let TexthookerEvent::Sentence(sentence) = texthooker_event {
    //                 send_sentence.send(sentence).await.ok()?;
    //             }
    //         }
    //     }
    //     .await;
    // });
    // TODO: forward server sentence events to overlay

    Ok(AppInit { engine, popup })
}

fn setup_profile_action(app: &adw::Application, engine: Engine) {
    let profiles = engine.profiles.load();
    let action = gio::ActionEntry::builder(ACTION_PROFILE)
        .parameter_type(Some(glib::VariantTy::STRING))
        .state(format!("{}", profiles.current_id.0).to_variant())
        .activate(move |_, action, param| {
            let profile_id = param
                .expect("activation should have parameter")
                .get::<String>()
                .expect("parameter should be a string")
                .parse::<i64>()
                .expect("parameter should be a valid integer");
            action.set_state(&format!("{profile_id}").into());

            let engine = engine.clone();
            glib::spawn_future_local(async move {
                if let Err(err) = engine.set_current_profile(ProfileId(profile_id)).await {
                    // todo: app-level notif toast and error handling
                    error!("Failed to set current profile: {err:?}");
                }
            });
        })
        .build();
    app.add_action_entries([action]);
}

const CHANNEL_BUF_CAP: usize = 4;
