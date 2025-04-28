use {
    crate::{
        AppEvent, MANAGE_PROFILES, PROFILE, anki_group::AnkiGroup,
        dictionary_group::DictionaryGroup, engine, forward_events, gettext, profile_row,
    },
    adw::prelude::*,
    glib::clone,
    relm4::prelude::*,
    wordbase_engine::EngineEvent,
};

mod ui;

#[derive(Debug)]
pub struct Manager {
    _dictionary_group: AsyncController<DictionaryGroup>,
    _anki_group: AsyncController<AnkiGroup>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    SearchChanged,
}

impl AsyncComponent for Manager {
    type Init = ();
    type Input = Msg;
    type Output = anyhow::Error;
    type CommandOutput = AppEvent;
    type Root = ui::Manager;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::Manager::new()
    }

    async fn init(
        (): Self::Init,
        ui: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);
        Self::update_profiles(&ui);
        ui.search_entry().grab_focus();

        let settings_page = ui.advanced().parent().expect("should have parent");

        let dictionary_group = DictionaryGroup::builder()
            .launch(())
            .forward(sender.output_sender(), |resp| resp);

        dictionary_group
            .widget()
            .insert_before(&settings_page, Some(&ui.advanced()));

        let anki_group = AnkiGroup::builder()
            .launch(())
            .forward(sender.output_sender(), |resp| resp);
        anki_group
            .widget()
            .insert_after(&settings_page, Some(&ui.advanced()));

        ui.quit().connect_activated(move |_| {
            relm4::main_application().quit();
        });

        Self::update_content(&ui);
        ui.search_entry().connect_search_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SearchChanged)
        ));

        AsyncComponentParts {
            model: Self {
                _dictionary_group: dictionary_group,
                _anki_group: anki_group,
            },
            widgets: (),
        }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        ui: &Self::Root,
    ) {
        match msg {
            Msg::SearchChanged => {
                Self::update_content(ui);
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
            AppEvent::Engine(
                EngineEvent::ProfileAdded { .. }
                | EngineEvent::ProfileRemoved { .. }
                | EngineEvent::ProfileNameSet { .. },
            ) => {
                Self::update_profiles(root);
            }
            _ => {}
        }
    }
}

impl Manager {
    fn update_content(ui: &ui::Manager) {
        let pages = ui
            .content_stack()
            .pages()
            .downcast::<adw::ViewStackPages>()
            .expect("should be `adw::ViewStackPages`");

        let next_page = if engine().dictionaries().len() == 0 {
            ui.page_no_dictionaries()
        } else if ui.search_entry().text().is_empty() {
            ui.page_landing()
        } else {
            ui.page_lookup()
        };
        pages.set_selected_page(&next_page);
    }

    fn update_profiles(ui: &ui::Manager) {
        ui.profile_menu().remove_all();
        for (profile_id, profile) in engine().profiles().iter() {
            let name = profile_row::name_of(profile);
            let action = format!("app.{PROFILE}::{}", profile_id.0);
            ui.profile_menu().append(Some(&name), Some(&action));
        }

        ui.profile_menu().append(
            Some(gettext("Manage Profiles")),
            Some(&format!("app.{MANAGE_PROFILES}")),
        );
    }
}
