use std::sync::Arc;

use adw::prelude::*;
use glib::clone;
use relm4::prelude::*;
use tokio_util::task::AbortOnDropHandle;
use wordbase_engine::{EngineEvent, anki::AnkiConfig};

use crate::{AppEvent, MANAGE_PROFILES, PROFILE, engine, forward_events, gettext};

mod ui;

#[derive(Debug)]
pub struct Manager {
    connect_anki_task: Option<AbortOnDropHandle<()>>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    ConnectAnki,
    UpdateConnectAnki,
}

impl AsyncComponent for Manager {
    type Init = ();
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::Manager;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::Manager::new()
    }

    async fn init(
        (): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);
        Self::update_profiles(&root);

        root.quit().connect_activated(move |_| {
            relm4::main_application().quit();
        });

        let anki_config = engine().anki_config();
        root.ankiconnect_server_url()
            .set_text(&anki_config.server_url);
        root.ankiconnect_server_url().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::ConnectAnki)
        ));
        root.ankiconnect_api_key().set_text(&anki_config.api_key);
        root.ankiconnect_api_key().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::ConnectAnki)
        ));

        Self::update_ankiconnect(&root);
        AsyncComponentParts {
            model: Self {
                connect_anki_task: None,
            },
            widgets: (),
        }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            Msg::ConnectAnki => {
                let server_url = root.ankiconnect_server_url().text();
                let api_key = root.ankiconnect_api_key().text();
                self.connect_anki_task = Some(AbortOnDropHandle::new(tokio::spawn(async move {
                    _ = engine()
                        .connect_anki(Arc::new(AnkiConfig {
                            server_url: Arc::from(server_url.to_string()),
                            api_key: Arc::from(api_key.to_string()),
                        }))
                        .await;
                    sender.input(Msg::UpdateConnectAnki);
                })));
            }
            Msg::UpdateConnectAnki => {
                Self::update_ankiconnect(root);
            }
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        event: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match event {
            AppEvent::Engine(EngineEvent::Profile(_)) => {
                Self::update_profiles(root);
            }
            _ => {}
        }
    }
}

impl Manager {
    fn update_profiles(root: &ui::Manager) {
        root.profile_menu().remove_all();
        for (profile_id, profile) in engine().profiles().iter() {
            let name = profile
                .config
                .name
                .as_ref()
                .map_or_else(|| gettext("Default Profile"), |s| s.as_str());
            let action = format!("app.{PROFILE}::{}", profile_id.0);
            root.profile_menu().append(Some(name), Some(&action));
        }

        root.profile_menu().append(
            Some(gettext("Manage Profiles")),
            Some(&format!("app.{MANAGE_PROFILES}")),
        );
    }

    fn update_ankiconnect(root: &ui::Manager) {
        match engine().anki_state() {
            Ok(_) => {
                root.ankiconnect_connected().set_visible(true);
                root.ankiconnect_disconnected().set_visible(false);
            }
            Err(err) => {
                root.ankiconnect_connected().set_visible(false);
                root.ankiconnect_disconnected().set_visible(true);
                root.ankiconnect_disconnected()
                    .set_tooltip_text(Some(&format!("{err:?}")));
            }
        }
    }
}
