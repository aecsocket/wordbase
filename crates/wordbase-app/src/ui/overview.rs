use adw::{glib, gtk, prelude::*, subclass::prelude::*};
use derive_more::{Deref, DerefMut};
use wordbase::DictionaryState;

use crate::{DictionaryImportState, ThemeMeta};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/overview.blp")]
    pub struct Overview {
        #[template_child]
        pub dictionaries: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub themes: TemplateChild<adw::PreferencesGroup>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Overview {
        const NAME: &str = "Overview";
        type Type = super::Overview;
        type ParentType = adw::BreakpointBin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Overview {}
    impl WidgetImpl for Overview {}
    impl BreakpointBinImpl for Overview {}
}

glib::wrapper! {
    pub struct Overview(ObjectSubclass<imp::Overview>) @extends gtk::Widget, adw::BreakpointBin;
}

impl Overview {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn dictionaries(&self) -> adw::PreferencesGroup {
        self.imp().dictionaries.get()
    }

    #[must_use]
    pub fn themes(&self) -> adw::PreferencesGroup {
        self.imp().themes.get()
    }
}

#[derive(Debug, Clone)]
struct MetaGrid {
    grid: gtk::Grid,
    row: i32,
}

impl MetaGrid {
    fn new() -> Self {
        let grid = gtk::Grid::builder()
            .row_spacing(16)
            .column_spacing(16)
            .margin_top(16)
            .margin_bottom(16)
            .margin_start(16)
            .margin_end(16)
            .build();
        Self { grid, row: 0 }
    }

    fn has_content(&self) -> bool {
        self.row > 0
    }

    fn add(&mut self, key: &str, value: &str) {
        let key_label = gtk::Label::builder()
            .label(key)
            .halign(gtk::Align::End)
            .valign(gtk::Align::Start)
            .css_classes(["dim-label"])
            .build();
        self.grid.attach(&key_label, 0, self.row, 1, 1);

        let value_label = gtk::Label::builder()
            .label(value)
            .halign(gtk::Align::Start)
            .valign(gtk::Align::Start)
            .wrap(true)
            .build();
        self.grid.attach(&value_label, 1, self.row, 1, 1);
        self.row += 1;
    }

    fn add_many(&mut self, key_one: &str, key_many: &str, values: &[impl AsRef<str>]) {
        match (values.first(), values.len()) {
            (None, _) => {}
            (Some(value), 1) => {
                self.add(key_one, value.as_ref());
            }
            (_, _) => {
                self.add(
                    key_many,
                    &values
                        .iter()
                        .map(|value| format!("• {}", value.as_ref()))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            }
        }
    }
}

pub fn dictionary_row(dictionary: &DictionaryState) -> adw::ExpanderRow {
    let ui = adw::ExpanderRow::builder()
        .title(&dictionary.meta.name)
        .subtitle(&dictionary.meta.version)
        .build();

    let drag_handle = gtk::Image::builder()
        .icon_name("list-drag-handle-symbolic")
        .build();
    ui.add_prefix(&drag_handle);

    let enabled = gtk::Switch::builder()
        .valign(gtk::Align::Center)
        .active(dictionary.enabled)
        .build();
    ui.add_suffix(&enabled);

    let mut meta_grid = MetaGrid::new();

    meta_grid.add_many("Author", "Authors", &dictionary.meta.authors);

    if let Some(description) = &dictionary.meta.description {
        meta_grid.add("Description", description);
    }

    if meta_grid.has_content() {
        ui.add_row(&meta_grid.grid);
    }

    let action_row = adw::ActionRow::new();
    ui.add_row(&action_row);

    if let Some(url) = &dictionary.meta.url {
        let visit_url = gtk::Button::builder()
            .label("Visit Website")
            .valign(gtk::Align::Center)
            .build();
        action_row.add_suffix(&visit_url);
    }

    let delete = gtk::Button::builder()
        .label("Delete")
        .css_classes(["destructive-action"])
        .valign(gtk::Align::Center)
        .build();
    action_row.add_suffix(&delete);

    ui
}

pub fn dictionary_import_row(dictionary: &DictionaryImportState) -> adw::ActionRow {
    let ui = adw::ActionRow::new();

    ui.add_prefix(&adw::Spinner::new());

    let progress_bar = gtk::LevelBar::builder()
        .hexpand(true)
        .valign(gtk::Align::Center)
        .build();
    ui.add_suffix(&progress_bar);

    match dictionary {
        DictionaryImportState::ReadingMeta { file_name } => {
            ui.set_title(file_name);
        }
        DictionaryImportState::Parsing { meta, total, done } => {
            ui.set_title(&meta.name);
            ui.set_subtitle(&meta.version);

            let done_frac = (*done as f64) / (*total as f64);
            progress_bar.set_value(done_frac * 0.5);
        }
        DictionaryImportState::Inserting { meta, total, done } => {
            ui.set_title(&meta.name);
            ui.set_subtitle(&meta.version);

            let done_frac = (*done as f64) / (*total as f64);
            progress_bar.set_value(0.5 + done_frac * 0.5);
        }
    }

    let delete = gtk::Button::builder()
        .icon_name("user-trash-symbolic")
        .valign(gtk::Align::Center)
        .css_classes(["destructive-action", "flat"])
        .build();
    ui.add_suffix(&delete);

    ui
}

pub fn theme_row<const USER: bool>(theme: &ThemeMeta) -> (adw::ExpanderRow, gtk::CheckButton) {
    let ui = adw::ExpanderRow::builder()
        .title(&theme.name)
        .subtitle(&theme.version)
        .build();

    let selected = gtk::CheckButton::new();
    ui.add_prefix(&selected);

    let mut meta_grid = MetaGrid::new();

    meta_grid.add_many("Author", "Authors", &theme.authors);

    if let Some(description) = &theme.description {
        meta_grid.add("Description", description);
    }

    if meta_grid.has_content() {
        ui.add_row(&meta_grid.grid);
    }

    let action_row = adw::ActionRow::new();

    if let Some(url) = &theme.url {
        let visit_url = gtk::Button::builder()
            .label("Visit Website")
            .valign(gtk::Align::Center)
            .build();
        action_row.add_suffix(&visit_url);
    }

    let mut has_action = false;
    if USER {
        let delete = gtk::Button::builder()
            .label("Delete")
            .css_classes(["destructive-action"])
            .valign(gtk::Align::Center)
            .build();
        action_row.add_suffix(&delete);
        has_action = true;
    }

    if has_action {
        ui.add_row(&action_row);
    }

    (ui, selected)
}
