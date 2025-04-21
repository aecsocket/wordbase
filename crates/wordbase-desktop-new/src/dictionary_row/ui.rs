use relm4::adw::{self, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/dictionary_row/ui.blp")]
    pub struct DictionaryRow {
        #[template_child]
        pub imported: TemplateChild<gtk::Box>,
        #[template_child]
        pub enabled: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub importing: TemplateChild<adw::Bin>,
        #[template_child]
        pub import_error: TemplateChild<gtk::Button>,
        #[template_child]
        pub is_sorting: TemplateChild<gtk::Button>,
        #[template_child]
        pub progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub action_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub set_sorting: TemplateChild<gtk::Button>,
        #[template_child]
        pub visit_website: TemplateChild<gtk::Button>,
        #[template_child]
        pub remove: TemplateChild<gtk::Button>,
        #[template_child]
        pub remove_dialog: TemplateChild<adw::AlertDialog>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DictionaryRow {
        const NAME: &str = "WdbDictionaryRow";
        type Type = super::DictionaryRow;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DictionaryRow {}
    impl WidgetImpl for DictionaryRow {}
    impl ListBoxRowImpl for DictionaryRow {}
    impl PreferencesRowImpl for DictionaryRow {}
    impl ExpanderRowImpl for DictionaryRow {}
}

glib::wrapper! {
    pub struct DictionaryRow(ObjectSubclass<imp::DictionaryRow>) @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow;
}

impl DictionaryRow {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn imported(&self) -> gtk::Box {
        self.imp().imported.get()
    }

    #[must_use]
    pub fn enabled(&self) -> gtk::CheckButton {
        self.imp().enabled.get()
    }

    #[must_use]
    pub fn importing(&self) -> adw::Bin {
        self.imp().importing.get()
    }

    #[must_use]
    pub fn import_error(&self) -> gtk::Button {
        self.imp().import_error.get()
    }

    #[must_use]
    pub fn is_sorting(&self) -> gtk::Button {
        self.imp().is_sorting.get()
    }

    #[must_use]
    pub fn progress(&self) -> gtk::ProgressBar {
        self.imp().progress.get()
    }

    #[must_use]
    pub fn action_row(&self) -> adw::ActionRow {
        self.imp().action_row.get()
    }

    #[must_use]
    pub fn set_sorting(&self) -> gtk::Button {
        self.imp().set_sorting.get()
    }

    #[must_use]
    pub fn visit_website(&self) -> gtk::Button {
        self.imp().visit_website.get()
    }

    #[must_use]
    pub fn remove(&self) -> gtk::Button {
        self.imp().remove.get()
    }

    #[must_use]
    pub fn remove_dialog(&self) -> adw::AlertDialog {
        self.imp().remove_dialog.get()
    }
}
