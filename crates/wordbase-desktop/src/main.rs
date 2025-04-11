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
mod record_view;
mod theme;

mod icon_names {
    include!(concat!(env!("OUT_DIR"), "/icon_names.rs"));
}

use {
    anyhow::{Context, Result},
    derive_more::Debug,
    directories::ProjectDirs,
    foldhash::HashMap,
    platform::Platform,
    relm4::{
        MessageBroker,
        adw::{self, gio, prelude::*},
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    std::sync::{Arc, LazyLock},
    theme::{CustomTheme, ThemeName},
    tokio::{fs, sync::broadcast},
    tracing::{error, info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::ProfileId,
    wordbase_engine::{Engine, profile::ProfileConfig},
};

const APP_ID: &str = "io.github.aecsocket.Wordbase";
static APP_BROKER: MessageBroker<AppMsg> = MessageBroker::new();
static APP_EVENTS: LazyLock<broadcast::Sender<AppEvent>> =
    LazyLock::new(|| broadcast::channel(CHANNEL_BUF_CAP).0);

#[derive(Debug, Clone)]
pub enum AppEvent {
    FontSet,
}

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

    RelmApp::new(APP_ID)
        .with_broker(&APP_BROKER)
        .run_async::<App>(());
}

#[derive(Debug)]
struct App {
    themes: HashMap<ThemeName, CustomTheme>,
    manager: AsyncController<manager::Model>,
    overlays: AsyncController<overlay::Overlays>,
    main_popup: AsyncController<popup::Model>,
    engine: Engine,
    _theme_watcher: notify::RecommendedWatcher,
}

#[derive(Debug)]
enum AppMsg {
    ThemeInsert(CustomTheme),
    ThemeRemove(ThemeName),
    SetFont {
        font_family: Option<String>,
        font_face: Option<String>,
    },
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = AppMsg;
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::Window {
            model.manager.widget(),
        }
    }

    fn init_loading_widgets(root: Self::Root) -> Option<LoadingWidgets> {
        let settings = gio::Settings::new(APP_ID);
        settings
            .bind("manager-width", &root, "default-width")
            .build();
        settings
            .bind("manager-height", &root, "default-height")
            .build();
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
        let platform = Arc::<dyn Platform>::from(
            platform::default()
                .await
                .expect("failed to create platform"),
        );
        let (engine, initial_themes, theme_watcher) = init_engine(sender.input_sender().clone())
            .await
            .expect("failed to initialize engine");
        let custom_theme = None; // TODO
        setup_profile_action(engine.clone());

        let main_popup = popup::connector(&platform, engine.clone(), custom_theme.clone())
            .await
            .expect("failed to create popup")
            .detach();
        let model = Self {
            themes: initial_themes,
            manager: manager::Model::builder()
                .launch((root.clone(), engine.clone(), custom_theme))
                .detach(),
            overlays: overlay::Overlays::builder()
                .launch((engine.clone(), platform, main_popup.sender().clone()))
                .detach(),
            main_popup,
            engine,
            _theme_watcher: theme_watcher,
        };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _sender: AsyncComponentSender<Self>,
        _root: &Self::Root,
    ) {
        if let Err(err) = update(&self.engine, msg).await {
            // todo manager toast msg
            error!("{err:?}");
        }
    }
}

async fn update(engine: &Engine, msg: AppMsg) -> Result<()> {
    match msg {
        AppMsg::SetFont {
            font_family,
            font_face,
        } => {
            let profiles = engine.profiles();
            let current_profile = profiles
                .by_id
                .get(&profiles.current_id)
                .context("no current profile")?;
            engine
                .set_profile_config(
                    profiles.current_id,
                    &ProfileConfig {
                        font_family,
                        font_face,
                        ..current_profile.config.clone()
                    },
                )
                .await
                .context("failed to set profile config")?;
        }
        _ => {}
    }
    Ok(())
}

async fn init_engine(
    sender: relm4::Sender<AppMsg>,
) -> Result<(
    Engine,
    HashMap<ThemeName, CustomTheme>,
    notify::RecommendedWatcher,
)> {
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

    let (initial_themes, theme_watcher) = theme::watch_themes(data_path)
        .await
        .context("failed to start watching theme files")?;

    Ok((engine, initial_themes, theme_watcher))
}

fn setup_profile_action(engine: Engine) {
    let app = relm4::main_application();

    let action = gio::ActionEntry::builder(ACTION_PROFILE)
        .parameter_type(Some(glib::VariantTy::STRING))
        .state(format!("{}", engine.profiles().current_id.0).to_variant())
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

#[derive(Debug)]
struct SignalHandler {
    object: glib::Object,
    id: Option<glib::SignalHandlerId>,
}

impl Drop for SignalHandler {
    fn drop(&mut self) {
        self.object.disconnect(
            self.id
                .take()
                .expect("signal handler id should not be taken before drop"),
        );
    }
}

impl SignalHandler {
    pub fn new<T: IsA<glib::Object>>(
        object: &T,
        make_id: impl FnOnce(&T) -> glib::SignalHandlerId,
    ) -> Self {
        let id = make_id(object);
        Self {
            object: object.upcast_ref().clone(),
            id: Some(id),
        }
    }
}

const ACTION_PROFILE: &str = "profile";
