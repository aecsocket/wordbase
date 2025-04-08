use super::ui;
use glib::clone;
use relm4::{
    adw::{gio, prelude::*},
    prelude::*,
    view,
};
use wordbase::Dictionary;

use crate::gettext;

#[derive(Debug)]
pub struct Model {
    window: adw::Window,
    dictionary: Dictionary,
    sorting: bool,
}

#[derive(Debug)]
pub struct Config {
    pub window: adw::Window,
    pub dictionary: Dictionary,
    pub is_sorting: bool,
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

#[derive(Debug)]
pub struct Widgets {
    root: ui::DictionaryRow,
    meta_rows: Vec<gtk::Widget>,
}

impl Component for Model {
    type Init = Config;
    type Input = Msg;
    type Output = Response;
    type CommandOutput = ();
    type Root = ui::DictionaryRow;
    type Widgets = Widgets;

    fn init_root() -> Self::Root {
        ui::DictionaryRow::new()
    }

    fn init(
        config: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Self {
            window: config.window,
            dictionary: config.dictionary,
            sorting: config.is_sorting,
        };

        root.enabled().connect_toggled(clone!(
            #[strong]
            sender,
            move |button| {
                _ = sender.output(Response::SetEnabled(button.is_active()));
            }
        ));
        root.is_sorting().connect_toggled(clone!(
            #[strong]
            sender,
            move |button| {
                _ = sender.output(Response::SetSorting(button.is_active()));
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

        let mut widgets = Widgets {
            root,
            meta_rows: Vec::new(),
        };
        model.update_view(&mut widgets, sender);
        ComponentParts { model, widgets }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        let root = &widgets.root;
        root.imported().set_visible(true);
        root.importing().set_visible(false);
        root.import_error().set_visible(false);
        root.progress().set_visible(false);

        root.enabled().set_active(self.dictionary.enabled);
        root.is_sorting().set_active(self.sorting);

        let meta = &self.dictionary.meta;
        root.set_title(&meta.name);
        root.set_subtitle(meta.version.as_deref().unwrap_or_default());

        let parent = root
            .action_row()
            .parent()
            .expect("container should have parent")
            .downcast::<gtk::ListBox>()
            .expect("container parent should be a `ListBox`");
        for row in widgets.meta_rows.drain(..) {
            parent.remove(&row);
        }

        let mut add_meta_row = |key: &str, value: &str| {
            view! {
                #[name(meta_row)]
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
                            set_label: key,
                            set_xalign: 0.0,
                            set_yalign: 0.0,
                            set_wrap: true,
                            set_css_classes: &["caption", "dimmed"],
                        },

                        gtk::Label {
                            set_label: value,
                            set_xalign: 0.0,
                            set_yalign: 0.0,
                            set_wrap: true,
                            set_selectable: true,
                        },
                    }
                }
            };

            parent.insert(&meta_row, root.action_row().index());
            widgets.meta_rows.push(meta_row.upcast());
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
