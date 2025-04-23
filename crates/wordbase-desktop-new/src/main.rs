//! TODO
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]
#![allow(clippy::wildcard_imports, reason = "used for `imp` modules")]
#![allow(
    clippy::new_without_default,
    reason = "`gtk` doesn't follow this convention"
)]

extern crate gtk4 as gtk;
extern crate libadwaita as adw;
extern crate webkit6 as webkit;

mod anki_group;
mod dictionary_group;
mod dictionary_row;
mod error_page;
mod manage_profiles;
mod manager;
mod profile_row;
mod theme;
// mod theme_group;
// mod theme_row;
mod util;
// mod record_view;

mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

use {
    adw::prelude::*,
    anyhow::{Context, Result, anyhow},
    arc_swap::ArcSwap,
    derive_more::Debug,
    error_page::ErrorPage,
    glib::clone,
    manage_profiles::ManageProfiles,
    manager::Manager,
    relm4::{MessageBroker, RelmApp, loading_widgets::LoadingWidgets, prelude::*, view},
    std::{
        cell::OnceCell,
        sync::{Arc, LazyLock, OnceLock},
    },
    theme::{CustomTheme, ThemeName},
    tokio::sync::broadcast,
    tracing::{error, level_filters::LevelFilter, warn},
    tracing_subscriber::EnvFilter,
    wordbase::{Profile, ProfileId},
    wordbase_engine::{Engine, EngineEvent},
    wordbase_server::HTTP_PORT,
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

    RelmApp::new(APP_ID)
        .visible_on_activate(false)
        .with_broker(&APP_BROKER)
        .run_async::<App>(());
}

static APP_BROKER: MessageBroker<AppMsg> = MessageBroker::new();
static EVENTS: LazyLock<broadcast::Sender<AppEvent>> = LazyLock::new(|| broadcast::channel(16).0);

static ENGINE: OnceLock<Engine> = OnceLock::new();

fn engine() -> &'static Engine {
    ENGINE.get().expect("engine should be initialized")
}

thread_local! {
    static APP_WINDOW: OnceCell<gtk::Window> = const { OnceCell::new() };
}

fn app_window() -> gtk::Window {
    APP_WINDOW.with(|window_cell| {
        let window = window_cell.get().expect("window should be initialized");
        window.clone()
    })
}

static CURRENT_PROFILE_ID: OnceLock<ArcSwap<ProfileId>> = OnceLock::new();

fn current_profile() -> Arc<Profile> {
    let profile_id = CURRENT_PROFILE_ID
        .get()
        .expect("current profile should be initialized")
        .load();
    engine()
        .profiles()
        .get(&**profile_id)
        .cloned()
        .unwrap_or_else(|| {
            let default = engine()
                .profiles()
                .values()
                .next()
                .cloned()
                .expect("at least one profile should exist");
            settings()
                .set(PROFILE, default.id.0.to_string())
                .expect("failed to reset profile ID");
            warn!(
                "Profile {profile_id:?} does not exist, reset to {:?}",
                default.id
            );
            default
        })
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

#[derive(Debug)]
struct App {
    toaster: adw::ToastOverlay,
    manage_profiles: Option<AsyncController<ManageProfiles>>,
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
    ThemeAdded(CustomTheme),
    ThemeRemoved(ThemeName),
    ProfileIdSet,
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
        view! {
            #[local]
            root {
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
        let toaster = adw::ToastOverlay::new();
        root.set_content(Some(&toaster));
        APP_WINDOW.with(move |cell| {
            cell.set(root.upcast())
                .expect("window should not already be set");
        });

        match init(sender.clone()).await {
            Ok(()) => {
                let manager = Manager::builder()
                    .launch(())
                    .forward(sender.input_sender(), AppMsg::Error);
                toaster.set_child(Some(manager.widget()));
                Box::leak(Box::new(manager));
            }
            Err(err) => {
                let error_page = ErrorPage::builder().launch(err).detach();
                toaster.set_child(Some(error_page.widget()));
            }
        };

        AsyncComponentParts {
            model: Self {
                toaster,
                manage_profiles: None,
            },
            widgets: (),
        }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppMsg::Error(err) => {
                self.toaster.add_toast(adw::Toast::new(&err.to_string()));
                error!("{err:?}");
            }
            AppMsg::FatalError(err) => {
                let error_page = ErrorPage::builder().launch(err).detach();
                root.set_content(Some(error_page.widget()));
            }
            AppMsg::OpenManageProfiles => {
                let manage_profiles = ManageProfiles::builder()
                    .launch(())
                    .forward(sender.input_sender(), AppMsg::Error);
                adw::Dialog::builder()
                    .child(manage_profiles.widget())
                    .title(gettext("Manage Profiles"))
                    .width_request(400)
                    .height_request(600)
                    .build()
                    .present(Some(root));
                self.manage_profiles = Some(manage_profiles);
            }
        }
    }
}

async fn init(sender: AsyncComponentSender<App>) -> Result<()> {
    let data_dir = wordbase_engine::data_dir().context("failed to get data directory")?;
    let engine = Engine::new(&data_dir)
        .await
        .context("failed to create engine")?;
    ENGINE
        .set(engine.clone())
        .expect("engine should not already be set");

    setup_profile();

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

    let theme_file_watcher = theme::watch_themes(&data_dir)
        .await
        .context("failed to get initial themes")?;
    Box::leak(Box::new(theme_file_watcher));

    let app = relm4::main_application();
    let settings = settings();
    let action = settings.create_action(PROFILE);
    app.add_action(&action);

    let manage_profiles = gio::ActionEntryBuilder::new(MANAGE_PROFILES)
        .activate(clone!(
            #[strong]
            sender,
            move |_, _, _| sender.input(AppMsg::OpenManageProfiles)
        ))
        .build();
    app.add_action_entries([manage_profiles]);

    Ok(())
}

fn parse_profile_id(settings: &gio::Settings) -> ProfileId {
    let profile_str = settings.string(PROFILE);
    profile_str.parse::<i64>().map(ProfileId).unwrap_or_else(|_| {
        let default_id = *engine().profiles().keys().next().expect("at least one profile should exist");
        settings.set(PROFILE, default_id.0.to_string())
            .expect("failed to reset profile ID");
        warn!("Profile ID was {profile_str:?} which is not a valid integer, reset to {default_id:?}");
        default_id
    })
}

fn setup_profile() {
    let settings = gio::Settings::new(APP_ID);

    let profile_id = parse_profile_id(&settings);
    CURRENT_PROFILE_ID
        .set(ArcSwap::from_pointee(profile_id))
        .expect("profile should not already be initialized");

    settings.connect_changed(
        Some(PROFILE),
        clone!(
            #[strong]
            settings,
            move |_, _| {
                let profile_id = parse_profile_id(&settings);
                CURRENT_PROFILE_ID
                    .get()
                    .expect("profile should be initialized")
                    .store(Arc::new(profile_id));
                _ = EVENTS.send(AppEvent::ProfileIdSet);
            }
        ),
    );
}

const PROFILE: &str = "profile";
const MANAGE_PROFILES: &str = "manage-profiles";
const CUSTOM_THEME: &str = "custom-theme";
