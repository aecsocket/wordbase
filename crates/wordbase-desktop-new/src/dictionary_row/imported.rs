use {
    super::ui,
    crate::{
        AppEvent, app_window, current_profile, current_profile_id, engine, gettext,
        util::{AppComponent, impl_component},
    },
    anyhow::{Context, Result},
    arc_swap::ArcSwap,
    glib::clone,
    relm4::{
        adw::{gdk, gio, prelude::*},
        prelude::*,
    },
    std::sync::{
        Arc,
        atomic::{self, AtomicI32},
    },
    wordbase::Dictionary,
    wordbase_engine::EngineEvent,
};

#[derive(Debug)]
pub struct DictionaryRow {
    dictionary: Arc<ArcSwap<Dictionary>>,
    meta_rows: Vec<Controller<MetaRow>>,
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
    type Args = (Arc<ArcSwap<Dictionary>>, gtk::ListBox);
    type Msg = Msg;
    type Ui = ui::DictionaryRow;

    async fn init(
        (dictionary, list): Self::Args,
        ui: Self::Ui,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        ui.set_dictionary(dictionary.load().clone());

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
        ui.processing().set_visible(false);
        ui.import_error().set_visible(false);
        ui.progress().set_visible(false);

        let drag_source = gtk::DragSource::builder()
            .actions(gdk::DragAction::MOVE)
            .build();

        let (drag_x, drag_y) = (Arc::new(AtomicI32::new(0)), Arc::new(AtomicI32::new(0)));
        drag_source.connect_prepare(clone!(
            #[strong]
            ui,
            #[strong]
            drag_x,
            #[strong]
            drag_y,
            move |_, x, y| {
                drag_x.store(x as i32, atomic::Ordering::SeqCst);
                drag_y.store(y as i32, atomic::Ordering::SeqCst);
                let value = ui.to_value();
                Some(gdk::ContentProvider::for_value(&value))
            }
        ));
        drag_source.connect_drag_begin(clone!(
            #[strong]
            dictionary,
            #[strong]
            ui,
            #[strong]
            drag_x,
            #[strong]
            drag_y,
            move |_, drag| {
                drag.set_hotspot(
                    drag_x.load(atomic::Ordering::SeqCst),
                    drag_y.load(atomic::Ordering::SeqCst),
                );

                let icon_list = gtk::ListBox::new();
                // TODO: wtf is up with the width??
                icon_list.set_size_request(ui.width(), ui.height());
                icon_list.add_css_class("boxed-list");

                let icon_row = Self::builder()
                    .launch((dictionary.clone(), icon_list.clone()))
                    .detach()
                    .widget()
                    .clone();
                icon_list.append(&icon_row);
                icon_list.drag_highlight_row(&icon_row);

                let icon = gtk::DragIcon::for_drag(drag);
                icon.set_child(Some(&icon_list));
            }
        ));
        ui.add_controller(drag_source);

        let drop_controller = gtk::DropControllerMotion::new();
        drop_controller.connect_enter(clone!(
            #[strong]
            ui,
            #[strong]
            list,
            move |_, _x, _y| list.drag_highlight_row(&ui)
        ));
        drop_controller.connect_leave(clone!(
            #[strong]
            list,
            move |_| list.drag_unhighlight_row()
        ));
        ui.add_controller(drop_controller);

        let mut model = Self {
            dictionary,
            meta_rows: Vec::new(),
        };
        model.update_ui(&ui);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Self::Msg,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        let dictionary = self.dictionary.load();
        match msg {
            Msg::SetEnabled(false) => {
                engine()
                    .disable_dictionary(current_profile().id, dictionary.id)
                    .await
                    .with_context(|| gettext("Failed to disable dictionary"))?;
            }
            Msg::SetEnabled(true) => {
                engine()
                    .enable_dictionary(current_profile().id, dictionary.id)
                    .await
                    .with_context(|| gettext("Failed to enable dictionary"))?;
            }
            Msg::SetSorting(false) => {
                engine()
                    .set_sorting_dictionary(current_profile().id, None)
                    .await
                    .with_context(|| gettext("Failed to unset sorting dictionary"))?;
            }
            Msg::SetSorting(true) => {
                engine()
                    .set_sorting_dictionary(current_profile().id, Some(dictionary.id))
                    .await
                    .with_context(|| gettext("Failed to set sorting dictionary"))?;
            }
            Msg::VisitWebsite => {
                if let Some(url) = &dictionary.meta.url {
                    gtk::UriLauncher::new(url).launch(
                        Some(&app_window()),
                        None::<&gio::Cancellable>,
                        |_| {},
                    );
                }
            }
            Msg::AskRemove => {
                ui.remove_dialog().present(Some(&app_window()));
            }
            Msg::Remove => {
                ui.imported().set_visible(false);
                ui.processing().set_visible(true);

                engine()
                    .remove_dictionary(dictionary.id)
                    .await
                    .with_context(|| gettext("Failed to remove dictionary"))?;
            }
        }
        Ok(())
    }

    async fn update_event(
        &mut self,
        event: AppEvent,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        match event {
            AppEvent::ProfileIdSet => self.sync(ui),
            AppEvent::Engine(EngineEvent::SortingDictionarySet { profile_id, .. })
                if profile_id == current_profile_id() =>
            {
                self.sync(ui);
            }
            _ => {}
        }
        Ok(())
    }
}

impl DictionaryRow {
    fn sync(&mut self, ui: &ui::DictionaryRow) {
        if let Some(dictionary) = engine().dictionaries().get(&self.dictionary.load().id) {
            self.dictionary.store(dictionary.clone());
        }
        self.update_ui(ui);
    }

    fn update_ui(&mut self, ui: &ui::DictionaryRow) {
        let dictionary = self.dictionary.load();
        let meta = &dictionary.meta;
        ui.set_title(&meta.name);
        ui.set_subtitle(meta.version.as_deref().unwrap_or_default());
        ui.visit_website().set_visible(meta.url.is_some());

        for meta_row in self.meta_rows.drain(..) {
            ui.meta_list().remove(meta_row.widget());
        }

        self.meta_rows.push(new_meta_row(
            ui,
            gettext("Format"),
            format!("{:?}", meta.kind),
        ));
        if let Some(description) = &meta.description {
            if !description.trim().is_empty() {
                self.meta_rows
                    .push(new_meta_row(ui, gettext("Description"), description));
            }
        }
        if let Some(attribution) = &meta.attribution {
            if !attribution.trim().is_empty() {
                self.meta_rows
                    .push(new_meta_row(ui, gettext("Attribution"), attribution));
            }
        }

        let profile = current_profile();
        ui.enabled()
            .set_active(profile.enabled_dictionaries.contains(&dictionary.id));
        ui.is_sorting()
            .set_visible(profile.sorting_dictionary == Some(dictionary.id));
    }
}

fn new_meta_row(
    ui: &ui::DictionaryRow,
    key: impl Into<String>,
    value: impl Into<String>,
) -> Controller<MetaRow> {
    let row = MetaRow::builder()
        .launch((key.into(), value.into()))
        .detach();

    ui.meta_list().insert(row.widget(), ui.action_row().index());
    row
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
