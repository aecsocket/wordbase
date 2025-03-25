use adw::{glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/manager/dictionary_row.blp")]
    pub struct DictionaryRow {
        #[template_child]
        pub enabled_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub enabled: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub importing_bin: TemplateChild<adw::Bin>,
        #[template_child]
        pub import_error: TemplateChild<gtk::Button>,
        #[template_child]
        pub progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub visit_website: TemplateChild<gtk::Button>,
        #[template_child]
        pub delete: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DictionaryRow {
        const NAME: &str = "DictionaryRow";
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
    pub fn enabled_bin(&self) -> adw::Bin {
        self.imp().enabled_bin.get()
    }

    #[must_use]
    pub fn enabled(&self) -> gtk::CheckButton {
        self.imp().enabled.get()
    }

    #[must_use]
    pub fn importing_bin(&self) -> adw::Bin {
        self.imp().importing_bin.get()
    }

    #[must_use]
    pub fn import_error(&self) -> gtk::Button {
        self.imp().import_error.get()
    }

    #[must_use]
    pub fn progress(&self) -> gtk::ProgressBar {
        self.imp().progress.get()
    }

    #[must_use]
    pub fn visit_website(&self) -> gtk::Button {
        self.imp().visit_website.get()
    }

    #[must_use]
    pub fn delete(&self) -> gtk::Button {
        self.imp().delete.get()
    }
}
