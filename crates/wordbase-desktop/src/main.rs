#![doc = include_str!("../README.md")]
#![allow(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]
#![allow(clippy::wildcard_imports, reason = "used for `imp` modules")]
#![allow(
    clippy::new_without_default,
    reason = "`gtk` doesn't follow this convention"
)]

mod manager;
mod state;
// mod overlay;
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
    manager::Manager,
    platform::Platform,
    popup::Popup,
    relm4::{
        adw::{self, gio, prelude::*},
        loading_widgets::LoadingWidgets,
        prelude::*,
        view,
    },
    std::sync::Arc,
    tokio::fs,
    tracing::{error, info, level_filters::LevelFilter},
    tracing_subscriber::EnvFilter,
    wordbase::ProfileId,
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

    RelmApp::new(APP_ID).run_async::<App>(());
}

#[derive(Debug)]
struct App {
    manager: AsyncController<Manager>,
    main_popup: Option<AsyncController<Popup>>,
}

#[relm4::component(async)]
impl AsyncComponent for App {
    type Init = ();
    type Input = ();
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
        _sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let platform = Arc::<dyn Platform>::from(
            platform::default()
                .await
                .expect("failed to create platform"),
        );
        let engine = init_engine().await.expect("failed to initialize engine");
        setup_profile_action(engine.clone());

        let model = Self {
            manager: Manager::builder()
                .launch((root.clone(), engine.clone()))
                .detach(),
            main_popup: match popup::connector(&platform, engine).await {
                Ok(popup) => Some(popup.detach()),
                Err(err) => {
                    error!("Failed to create popup: {err:?}");
                    None
                }
            },
        };
        let widgets = view_output!();
        AsyncComponentParts { model, widgets }
    }
}

async fn init_engine() -> Result<Engine> {
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

    Ok(engine)
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
