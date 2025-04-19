//! TODO
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]
#![allow(clippy::wildcard_imports, reason = "used for `imp` modules")]
#![allow(
    clippy::new_without_default,
    reason = "`gtk` doesn't follow this convention"
)]

extern crate gtk4 as gtk;
extern crate libadwaita as adw;

mod error_page;
mod manage_profiles;
mod manager;
mod profile_row;

mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

use std::sync::{LazyLock, OnceLock};

use adw::prelude::*;
use anyhow::{Context, Result, anyhow};
use derive_more::Debug;
use error_page::ErrorPage;
use glib::clone;
use manage_profiles::ManageProfiles;
use manager::Manager;
use relm4::{MessageBroker, RelmApp, SharedState, loading_widgets::LoadingWidgets, prelude::*};
use tokio::sync::broadcast;
use tracing::{error, level_filters::LevelFilter};
use tracing_subscriber::EnvFilter;
use wordbase::ProfileId;
use wordbase_engine::{Engine, EngineEvent};
use wordbase_server::HTTP_PORT;

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

    RelmApp::new(APP_ID)
        .visible_on_activate(false)
        .with_broker(&APP_BROKER)
        .run_async::<App>(());
}

static ENGINE: OnceLock<Engine> = OnceLock::new();
static APP_BROKER: MessageBroker<AppMsg> = MessageBroker::new();
static CURRENT_PROFILE_ID: SharedState<ProfileId> = SharedState::new();
static EVENTS: LazyLock<broadcast::Sender<AppEvent>> = LazyLock::new(|| broadcast::channel(16).0);

fn engine() -> &'static Engine {
    ENGINE.get().expect("engine should be initialized")
}

#[must_use]
fn settings() -> gio::Settings {
    gio::Settings::new(APP_ID)
}

fn gettext(s: &str) -> &str {
    s
}

fn forward_events<C: AsyncComponent<CommandOutput = AppEvent>>(sender: &AsyncComponentSender<C>) {
    sender.command(|out, shutdown| {
        shutdown
            .register(async move {
                let mut recv_event = EVENTS.subscribe();
                while let Ok(event) = recv_event.recv().await {
                    if out.send(event).is_err() {
                        return;
                    }
                }
            })
            .drop_on_shutdown()
    });
}

fn handle_result<T>(result: Result<T>) {
    if let Err(err) = result {
        APP_BROKER.send(AppMsg::Error(err));
    }
}

#[derive(Debug)]
struct App {
    toaster: adw::ToastOverlay,
    manage_profiles: Option<AsyncController<ManageProfiles>>,
    _manager: Option<AsyncController<Manager>>,
}

#[derive(Debug)]
enum AppMsg {
    Error(anyhow::Error),
    FatalError(anyhow::Error),
    #[doc(hidden)]
    OpenManageProfiles,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
    Engine(EngineEvent),
    ProfileSet,
}

impl AsyncComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();
    type Root = adw::ApplicationWindow;
    type Widgets = ();

    fn init_root() -> Self::Root {
        let root = adw::ApplicationWindow::builder()
            .application(&relm4::main_application())
            .title("Wordbase")
            .build();

        let settings = settings();
        settings
            .bind("manager-width", &root, "default-width")
            .build();
        settings
            .bind("manager-height", &root, "default-height")
            .build();
        root.present();
        root
    }

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        None
    }

    async fn init(
        (): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let toaster = adw::ToastOverlay::new();
        root.set_content(Some(&toaster));

        let manager = match init(sender).await {
            Ok(engine) => {
                ENGINE
                    .set(engine)
                    .expect("engine should not already be set");
                let manager = Manager::builder().launch(root.clone()).detach();
                toaster.set_child(Some(manager.widget()));
                Some(manager)
            }
            Err(err) => {
                let error_page = ErrorPage::builder().launch(err).detach();
                toaster.set_child(Some(error_page.widget()));
                None
            }
        };

        AsyncComponentParts {
            model: Self {
                toaster,
                manage_profiles: None,
                _manager: manager,
            },
            widgets: (),
        }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppMsg::OpenManageProfiles => {
                let manage_profiles = ManageProfiles::builder()
                    .launch(root.clone().upcast())
                    .detach();
                adw::Dialog::builder()
                    .child(manage_profiles.widget())
                    .title(gettext("Manage Profiles"))
                    .width_request(400)
                    .height_request(600)
                    .build()
                    .present(Some(root));
                self.manage_profiles = Some(manage_profiles);
            }
            AppMsg::Error(err) => {
                self.toaster.add_toast(adw::Toast::new(&err.to_string()));
                error!("{err:?}");
            }
            AppMsg::FatalError(err) => {
                let error_page = ErrorPage::builder().launch(err).detach();
                root.set_content(Some(error_page.widget()));
            }
        }
    }
}

async fn init(sender: AsyncComponentSender<App>) -> Result<Engine> {
    let data_dir = wordbase_engine::data_dir().context("failed to get data directory")?;
    let engine = Engine::new(data_dir)
        .await
        .context("failed to create engine")?;

    tokio::spawn(clone!(
        #[strong]
        engine,
        #[strong]
        sender,
        async move {
            let addr = format!("127.0.0.1:{HTTP_PORT}");
            let err = match wordbase_server::run(engine, addr).await {
                Ok(()) => anyhow!("server exited"),
                Err(err) => err,
            }
            .context("server task failed");
            sender.input(AppMsg::FatalError(err));
        }
    ));

    tokio::spawn(clone!(
        #[strong]
        engine,
        async move {
            let mut recv_event = engine.recv_event();
            while let Ok(event) = recv_event.recv().await {
                _ = EVENTS.send(AppEvent::Engine(event));
            }
        }
    ));

    let app = relm4::main_application();
    let settings = settings();
    let action = settings.create_action(PROFILE);
    app.add_action(&action);

    if let Ok(profile_id) = settings.string(PROFILE).parse::<i64>().map(ProfileId) {
        *CURRENT_PROFILE_ID.write() = profile_id;
    }
    settings.connect_changed(
        Some(PROFILE),
        clone!(
            #[strong]
            settings,
            move |_, _| {
                let Ok(profile_id) = settings.string(PROFILE).parse::<i64>().map(ProfileId) else {
                    return;
                };
                *CURRENT_PROFILE_ID.write() = profile_id;
            }
        ),
    );

    let manage_profiles = gio::ActionEntryBuilder::new(MANAGE_PROFILES)
        .activate(clone!(
            #[strong]
            sender,
            move |_, _, _| sender.input(AppMsg::OpenManageProfiles)
        ))
        .build();
    app.add_action_entries([manage_profiles]);

    Ok(engine)
}

const PROFILE: &str = "profile";
const MANAGE_PROFILES: &str = "manage-profiles";
