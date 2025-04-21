use {
    super::ui,
    crate::{
        AppEvent, CURRENT_PROFILE_ID, app_window, current_profile_id, engine, gettext,
        util::{AppComponent, impl_component},
    },
    anyhow::{Context, Result},
    glib::clone,
    relm4::{
        adw::{gio, prelude::*},
        prelude::*,
    },
    wordbase::DictionaryId,
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

impl_component!(DictionaryRow);

impl AppComponent for DictionaryRow {
    type Args = DictionaryId;
    type Msg = Msg;
    type Ui = ui::DictionaryRow;

    async fn init(
        dictionary_id: Self::Args,
        ui: Self::Ui,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        ui.enabled().connect_toggled(clone!(
            #[strong]
            sender,
            move |button| sender.input(Msg::SetEnabled(button.is_active()))
        ));
        ui.is_sorting().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SetSorting(false))
        ));
        ui.set_sorting().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SetSorting(true))
        ));
        ui.remove().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskRemove)
        ));
        ui.remove_dialog().connect_response(
            Some("remove_confirm"),
            clone!(
                #[strong]
                sender,
                move |_, _| sender.input(Msg::Remove)
            ),
        );
        ui.visit_website().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::VisitWebsite)
        ));

        ui.imported().set_visible(true);
        ui.importing().set_visible(false);
        ui.import_error().set_visible(false);
        ui.progress().set_visible(false);

        let meta_parent = ui
            .action_row()
            .parent()
            .expect("action row should have parent")
            .downcast::<gtk::ListBox>()
            .expect("action row parent should be a `ListBox`");
        let add_meta_row = |key: &str, value: &str| {
            let row = MetaRow::builder()
                .launch((key.to_string(), value.to_string()))
                .detach();
            meta_parent.insert(row.widget(), ui.action_row().index());
        };

        let meta = &dictionary.meta;

        ui.set_title(&meta.name);
        ui.set_subtitle(meta.version.as_deref().unwrap_or_default());

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
        ui.visit_website().set_visible(meta.url.is_some());

        let model = Self { dictionary };
        show_enabled(&model, &ui);
        show_sorting(&model, &ui);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Self::Msg,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        match msg {
            Msg::SetEnabled(enabled) => set_enabled(self, enabled)
                .await
                .with_context(|| gettext("Failed to set dictionary enabled"))?,
            Msg::SetSorting(sorting) => set_sorting(self, sorting)
                .await
                .with_context(|| gettext("Failed to set sorting dictionary"))?,
            Msg::VisitWebsite => {
                visit_website(self).with_context(|| gettext("Failed to open website"))?
            }
            Msg::AskRemove => {
                ui.remove_dialog().present(Some(&app_window()));
            }
            Msg::Remove => remove(self)
                .await
                .with_context(|| gettext("Failed to remove dictionary"))?,
        }
        Ok(())
    }

    async fn update_event(
        &mut self,
        event: AppEvent,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) {
        match message {
            AppEvent::DictionaryEnabledSet(id, _) if id == self.dictionary.id => {
                sync(self);
                show_enabled(self, ui);
            }
            AppEvent::DictionarySortingSet(_) => {
                show_sorting(self, ui);
            }
            _ => {}
        }
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
    let sorting_id = if sorting {
        Some(model.dictionary_id)
    } else {
        None
    };
    engine()
        .set_sorting_dictionary(current_profile_id(), sorting_id)
        .await?;
    Ok(())
}

fn visit_website(model: &DictionaryRow) -> Result<()> {
    let dictionary = engine().dictionaries().get(model.dictionary_id)
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
