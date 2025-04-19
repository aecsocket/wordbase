use adw::prelude::*;
use relm4::prelude::*;
use wordbase_engine::Event;

use crate::{MANAGE_PROFILES, PROFILE, engine, forward_as_command, gettext};

mod ui;

#[derive(Debug)]
pub struct Manager {
    window: adw::ApplicationWindow,
}

impl AsyncComponent for Manager {
    type Init = adw::ApplicationWindow;
    type Input = ();
    type Output = ();
    type CommandOutput = Event;
    type Root = ui::Manager;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::Manager::new()
    }

    async fn init(
        window: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_as_command(&sender);
        update_profiles(&root);
        AsyncComponentParts {
            model: Self { window },
            widgets: (),
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
            Event::Profile(_) => update_profiles(root),
            _ => {}
        }
    }
}

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
