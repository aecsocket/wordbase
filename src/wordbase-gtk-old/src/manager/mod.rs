use {
    crate::{
        AppEvent, MANAGE_PROFILES, PROFILE,
        anki_group::AnkiGroup,
        current_profile_id,
        dictionary_group::DictionaryGroup,
        engine, forward_events, gettext, profile_row,
        record_view::{self, RecordView, SUPPORTED_RECORD_KINDS},
    },
    adw::prelude::*,
    anyhow::{Context, Result},
    glib::clone,
    relm4::prelude::*,
    tokio_util::task::AbortOnDropHandle,
    wordbase::RecordLookup,
    wordbase_engine::{EngineEvent, ProfileEvent},
};

mod ui;

#[derive(Debug)]
pub struct Manager {
    record_view: AsyncController<RecordView>,
    search_task: Option<AbortOnDropHandle<()>>,
    _dictionary_group: AsyncController<DictionaryGroup>,
    _anki_group: AsyncController<AnkiGroup>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    Search,
    SearchResult(Result<(Vec<RecordLookup>, usize)>),
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
            .insert_before(&settings_page, Some(&ui.advanced()));

        ui.quit().connect_activated(move |_| {
            relm4::main_application().quit();
        });

        Self::update_content(&ui);
        ui.search_entry().connect_search_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::Search)
        ));

        let record_view = RecordView::builder().launch(()).detach();
        ui.lookup_results().set_child(Some(record_view.widget()));

        AsyncComponentParts {
            model: Self {
                record_view,
                search_task: None,
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
            Msg::Search => {
                Self::update_content(ui);
                let sentence = ui.search_entry().text().to_string();
                let sender = sender.input_sender().clone();
                let task = tokio::spawn(search_task(sentence, sender));
                self.search_task = Some(AbortOnDropHandle::new(task));
            }
            Msg::SearchResult(Ok((records, max_chars_scanned))) => {
                ui.search_entry().select_region(0, max_chars_scanned as i32);
                _ = self
                    .record_view
                    .sender()
                    .send(record_view::Msg::Render { records });
            }
            Msg::SearchResult(Err(err)) => {
                _ = sender.output(err);
            }
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        event: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        if record_view::should_requery(&event) {
            sender.input(Msg::Search);
        }

        match event {
            AppEvent::Engine(EngineEvent::Profile(
                ProfileEvent::Added { .. }
                | ProfileEvent::Removed { .. }
                | ProfileEvent::NameSet { .. },
            )) => {
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

async fn search_task(sentence: String, sender: relm4::Sender<Msg>) {
    let result = engine()
        .lookup(current_profile_id(), &sentence, 0, SUPPORTED_RECORD_KINDS)
        .await
        .map(|records| {
            let max_bytes_scanned = records
                .iter()
                .map(|record| record.bytes_scanned)
                .max()
                .unwrap_or_default();
            let max_chars_scanned = sentence
                .get(..max_bytes_scanned)
                .map(|s| s.chars().count())
                .unwrap_or_default();
            (records, max_chars_scanned)
        })
        .with_context(|| gettext("Failed to perform lookup"));
    sender.emit(Msg::SearchResult(result));
}
