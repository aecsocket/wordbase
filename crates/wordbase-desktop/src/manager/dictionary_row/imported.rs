use super::ui;
use glib::clone;
use relm4::{
    adw::{gio, prelude::*},
    prelude::*,
};
use wordbase::Dictionary;

use crate::gettext;

#[derive(Debug)]
pub struct Model {
    window: adw::Window,
    dictionary: Dictionary,
    sorting: bool,
    meta_rows: Vec<Controller<MetaRow>>,
}

#[derive(Debug)]
pub enum Msg {
    SetEnabled(bool),
    SetSorting(bool),
    #[doc(hidden)]
    VisitWebsite,
    #[doc(hidden)]
    AskRemove,
}

#[derive(Debug)]
pub enum Response {
    SetEnabled(bool),
    SetSorting(bool),
    Remove,
}

impl Component for Model {
    type Init = (adw::Window, Dictionary, bool);
    type Input = Msg;
    type Output = Response;
    type CommandOutput = ();
    type Root = ui::DictionaryRow;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::DictionaryRow::new()
    }

    fn init(
        (window, dictionary, sorting): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            window,
            dictionary,
            sorting,
            meta_rows: Vec::new(),
        };
        let dictionary = &model.dictionary;
        let meta = &dictionary.meta;

        root.enabled().connect_toggled(clone!(
            #[strong]
            sender,
            move |button| {
                _ = sender.output(Response::SetEnabled(button.is_active()));
            }
        ));
        root.is_sorting().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                _ = sender.output(Response::SetSorting(false));
            }
        ));
        root.set_sorting().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| {
                _ = sender.output(Response::SetSorting(true));
            }
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
                move |_, _| {
                    _ = sender.output(Response::Remove);
                }
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

        root.enabled().set_active(dictionary.enabled);
        root.is_sorting().set_visible(sorting);

        root.set_title(&meta.name);
        root.set_subtitle(meta.version.as_deref().unwrap_or_default());

        let meta_parent = root
            .action_row()
            .parent()
            .expect("action row should have parent")
            .downcast::<gtk::ListBox>()
            .expect("action row parent should be a `ListBox`");
        let mut add_meta_row = |key: &str, value: &str| {
            let row = MetaRow::builder()
                .launch((key.to_string(), value.to_string()))
                .detach();
            meta_parent.insert(row.widget(), root.action_row().index());
            model.meta_rows.push(row);
        };

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

        ComponentParts { model, widgets: () }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, root: &Self::Root) {
        match message {
            Msg::SetEnabled(enabled) => {
                self.dictionary.enabled = enabled;
            }
            Msg::SetSorting(sorting) => {
                self.sorting = sorting;
            }
            Msg::VisitWebsite => {
                if let Some(url) = &self.dictionary.meta.url {
                    gtk::UriLauncher::new(url).launch(
                        None::<&gtk::Window>,
                        None::<&gio::Cancellable>,
                        |_| {},
                    );
                }
            }
            Msg::AskRemove => {
                root.remove_dialog().present(Some(&self.window));
            }
        }
    }
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
