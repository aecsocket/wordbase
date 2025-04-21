use {
    crate::{
        AppEvent, MANAGE_PROFILES, PROFILE, anki_group::AnkiGroup,
        dictionary_group::DictionaryGroup, engine, forward_events, gettext,
    },
    adw::prelude::*,
    relm4::prelude::*,
    wordbase_engine::EngineEvent,
};

mod ui;

#[derive(Debug)]
pub struct Manager {
    _dictionary_group: AsyncController<DictionaryGroup>,
    _anki_group: AsyncController<AnkiGroup>,
}

impl AsyncComponent for Manager {
    type Init = ();
    type Input = ();
    type Output = anyhow::Error;
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

        let settings_page = root.advanced().parent().expect("should have parent");

        let dictionary_group = DictionaryGroup::builder()
            .launch(())
            .forward(sender.output_sender(), |resp| resp);

        dictionary_group
            .widget()
            .insert_before(&settings_page, Some(&root.advanced()));

        let anki_group = AnkiGroup::builder()
            .launch(())
            .forward(sender.output_sender(), |resp| resp);
        anki_group
            .widget()
            .insert_after(&settings_page, Some(&root.advanced()));

        root.quit().connect_activated(move |_| {
            relm4::main_application().quit();
        });

        AsyncComponentParts {
            model: Self {
                _dictionary_group: dictionary_group,
                _anki_group: anki_group,
            },
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
}
