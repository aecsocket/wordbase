use {
    super::ui,
    crate::{AppEvent, AppResponse, CURRENT_PROFILE_ID, forward_events, gettext, toast_result},
    anyhow::{Context, Result},
    glib::clone,
    relm4::{
        adw::{gio, prelude::*},
        prelude::*,
    },
    std::sync::Arc,
    wordbase::{Dictionary, DictionaryId},
    wordbase_engine::Engine,
};

#[derive(Debug)]
pub struct DictionaryRow {
    dictionary_id: DictionaryId,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    SetEnabled(bool),
    SetSorting(bool),
    VisitWebsite,
    AskRemove,
    Remove,
}

impl AsyncComponent for DictionaryRow {
    type Init = DictionaryId;
    type Input = Msg;
    type Output = AppResponse;
    type CommandOutput = AppEvent;
    type Root = ui::DictionaryRow;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::DictionaryRow::new()
    }

    async fn init(
        dictionary_id: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);

        root.enabled().connect_toggled(clone!(
            #[strong]
            sender,
            move |button| sender.input(Msg::SetEnabled(button.is_active()))
        ));
        root.is_sorting().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SetSorting(false))
        ));
        root.set_sorting().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SetSorting(true))
        ));
        root.remove().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskRemove)
        ));
        root.remove_dialog().connect_response(
            Some("remove_confirm"),
            clone!(
                #[strong]
                sender,
                move |_, _| sender.input(Msg::Remove)
            ),
        );
        root.visit_website().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::VisitWebsite)
        ));

        root.imported().set_visible(true);
        root.importing().set_visible(false);
        root.import_error().set_visible(false);
        root.progress().set_visible(false);

        let meta_parent = root
            .action_row()
            .parent()
            .expect("action row should have parent")
            .downcast::<gtk::ListBox>()
            .expect("action row parent should be a `ListBox`");
        let add_meta_row = |key: &str, value: &str| {
            let row = MetaRow::builder()
                .launch((key.to_string(), value.to_string()))
                .detach();
            meta_parent.insert(row.widget(), root.action_row().index());
        };

        let meta = &dictionary.meta;

        root.set_title(&meta.name);
        root.set_subtitle(meta.version.as_deref().unwrap_or_default());

        add_meta_row(gettext("Format"), &format!("{:?}", meta.kind));
        if let Some(description) = &meta.description {
            if !description.trim().is_empty() {
                add_meta_row(gettext("Description"), description);
            }
        }
        if let Some(attribution) = &meta.attribution {
            if !attribution.trim().is_empty() {
                add_meta_row(gettext("Attribution"), attribution);
            }
        }
        root.visit_website().set_visible(meta.url.is_some());

        let model = Self {
            dictionary,
            engine,
            window,
            toaster,
        };
        show_enabled(&model, &root);
        show_sorting(&model, &root);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        message: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        let result = match message {
            Msg::SetEnabled(enabled) => set_enabled(self, enabled)
                .await
                .with_context(|| gettext("Failed to set dictionary enabled")),
            Msg::SetSorting(sorting) => set_sorting(self, sorting)
                .await
                .with_context(|| gettext("Failed to set sorting dictionary")),
            Msg::VisitWebsite => {
                visit_website(self).with_context(|| gettext("Failed to open website"))
            }
            Msg::AskRemove => {
                root.remove_dialog().present(Some(&self.window));
                Ok(())
            }
            Msg::Remove => remove(self)
                .await
                .with_context(|| gettext("Failed to remove dictionary")),
        };
        if let Err(err) = result {
            _ = sender.output(AppResponse::Error(err));
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppEvent::DictionaryEnabledSet(id, _) if id == self.dictionary.id => {
                sync(self);
                show_enabled(self, root);
            }
            AppEvent::DictionarySortingSet(_) => {
                show_sorting(self, root);
            }
            _ => {}
        }
    }
}

fn sync(model: &mut DictionaryRow) {
    if let Some(dictionary) = model.engine.dictionaries().get(&model.dictionary.id) {
        model.dictionary = dictionary.clone();
    }
}

fn show_enabled(model: &DictionaryRow, root: &ui::DictionaryRow) {
    let enabled = CURRENT_PROFILE
        .read()
        .as_ref()
        .unwrap()
        .enabled_dictionaries
        .contains(&model.dictionary.id);
    root.enabled().set_active(enabled);
}

fn show_sorting(model: &DictionaryRow, root: &ui::DictionaryRow) {
    let is_sorting = CURRENT_PROFILE
        .read()
        .as_ref()
        .unwrap()
        .config
        .sorting_dictionary
        == Some(model.dictionary.id);
    root.is_sorting().set_visible(is_sorting);
}

async fn set_enabled(model: &DictionaryRow, enabled: bool) -> Result<()> {
    if enabled {
        model
            .engine
            .enable_dictionary(CURRENT_PROFILE_ID.read().unwrap(), model.dictionary.id)
            .await?;
    } else {
        model
            .engine
            .disable_dictionary(CURRENT_PROFILE_ID.read().unwrap(), model.dictionary.id)
            .await?;
    }
    _ = APP_EVENTS.send(AppEvent::DictionaryEnabledSet(model.dictionary.id, enabled));
    Ok(())
}

async fn set_sorting(model: &DictionaryRow, sorting: bool) -> Result<()> {
    if sorting {
        model
            .engine
            .set_sorting_dictionary(
                CURRENT_PROFILE_ID.read().unwrap(),
                Some(model.dictionary.id),
            )
            .await?;
        _ = APP_EVENTS.send(AppEvent::DictionarySortingSet(Some(model.dictionary.id)));
    } else {
        model
            .engine
            .set_sorting_dictionary(CURRENT_PROFILE_ID.read().unwrap(), None)
            .await?;
        _ = APP_EVENTS.send(AppEvent::DictionarySortingSet(None));
    }
    Ok(())
}

fn visit_website(model: &DictionaryRow) -> Result<()> {
    let url = model.dictionary.meta.url.as_ref().context("no URL")?;
    gtk::UriLauncher::new(url).launch(None::<&gtk::Window>, None::<&gio::Cancellable>, |_| {});
    Ok(())
}

async fn remove(model: &DictionaryRow) -> Result<()> {
    model.engine.remove_dictionary(model.dictionary.id).await?;
    _ = APP_EVENTS.send(AppEvent::DictionaryRemoved(model.dictionary.id));
    Ok(())
}

#[derive(Debug)]
struct MetaRow {
    key: String,
    value: String,
}

#[relm4::component]
impl Component for MetaRow {
    type Init = (String, String);
    type Input = ();
    type Output = ();
    type CommandOutput = ();

    view! {
        adw::ActionRow {
            #[wrap(Some)]
            set_child = &gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 8,
                set_margin_start: 8,
                set_margin_end: 8,
                set_margin_top: 8,
                set_margin_bottom: 8,

                gtk::Label {
                    set_label: &model.key,
                    set_xalign: 0.0,
                    set_yalign: 0.0,
                    set_wrap: true,
                    set_css_classes: &["caption", "dimmed"],
                },

                gtk::Label {
                    set_label: &model.value,
                    set_xalign: 0.0,
                    set_yalign: 0.0,
                    set_wrap: true,
                    set_selectable: true,
                },
            }
        }
    }

    fn init(
        (key, value): Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self { key, value };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }
}
