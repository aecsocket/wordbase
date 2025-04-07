use gtk4::prelude::{GridExt, WidgetExt};
use relm4::{
    adw::prelude::{ExpanderRowExt, PreferencesRowExt},
    prelude::*,
    view,
};
use wordbase::{Dictionary, DictionaryMeta};

use crate::gettext;

mod ui;

#[derive(Debug)]
pub enum DictionaryRow {
    ImportingStart { file_path: String },
    Importing { meta: DictionaryMeta, progress: f64 },
    ImportingFailed { error: String },
    Imported(Dictionary),
}

impl Component for DictionaryRow {
    type Init = Self;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type Root = ui::DictionaryRow;
    type Widgets = ui::DictionaryRow;

    fn init_root() -> Self::Root {
        ui::DictionaryRow::new()
    }

    fn init(
        model: Self::Init,
        mut widgets: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        model.update_view(&mut widgets, sender);
        ComponentParts { model, widgets }
    }

    fn update_view(&self, widgets: &mut Self::Widgets, sender: ComponentSender<Self>) {
        match self {
            Self::ImportingStart { file_path } => {
                widgets.imported().set_visible(false);
                widgets.importing().set_visible(true);
                widgets.import_error().set_visible(false);

                widgets.progress().set_visible(true);
                widgets.progress().pulse();
            }
            Self::Importing { meta, progress } => {
                widgets.imported().set_visible(false);
                widgets.importing().set_visible(true);
                widgets.import_error().set_visible(false);

                widgets.progress().set_visible(true);
                show_meta(meta, widgets);
            }
            Self::ImportingFailed { error } => {
                widgets.imported().set_visible(false);
                widgets.importing().set_visible(false);
                widgets.import_error().set_visible(true);

                widgets.progress().set_visible(false);
                // todo connect click
            }
            Self::Imported(dictionary) => {
                widgets.imported().set_visible(true);
                widgets.importing().set_visible(false);
                widgets.import_error().set_visible(false);

                widgets.progress().set_visible(false);
                show_meta(&dictionary.meta, widgets);
            }
        }
    }
}

fn show_meta(meta: &DictionaryMeta, ui: &ui::DictionaryRow) {
    ui.set_title(&meta.name);
    ui.set_subtitle(meta.version.as_deref().unwrap_or_default());

    let mut row = 0i32;
    if let Some(description) = &meta.description {
        if !description.trim().is_empty() {
            let key = gtk::Label::new(Some("Description"));
            let value = gtk::Label::builder()
                .label(description)
                .wrap(true)
                .xalign(0.0)
                .build();

            ui.meta_info().attach(&key, 0, row, 1, 1);
            ui.meta_info().attach(&value, 1, row, 1, 1);
        }
    }
}
