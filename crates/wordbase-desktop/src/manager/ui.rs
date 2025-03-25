use adw::{glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/manager/ui.blp")]
    pub struct Manager {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub dictionaries: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub import_dictionary: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub themes: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub import_theme: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_content: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Manager {
        const NAME: &str = "Manager";
        type Type = super::Manager;
        type ParentType = adw::BreakpointBin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Manager {}
    impl WidgetImpl for Manager {}
    impl BreakpointBinImpl for Manager {}
}

glib::wrapper! {
    pub struct Manager(ObjectSubclass<imp::Manager>) @extends gtk::Widget, adw::BreakpointBin;
}

impl Manager {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn toast_overlay(&self) -> adw::ToastOverlay {
        self.imp().toast_overlay.get()
    }

    #[must_use]
    pub fn dictionaries(&self) -> gtk::ListBox {
        self.imp().dictionaries.get()
    }

    #[must_use]
    pub fn import_dictionary(&self) -> adw::ButtonRow {
        self.imp().import_dictionary.get()
    }

    #[must_use]
    pub fn themes(&self) -> adw::PreferencesGroup {
        self.imp().themes.get()
    }

    #[must_use]
    pub fn import_theme(&self) -> adw::ButtonRow {
        self.imp().import_theme.get()
    }

    #[must_use]
    pub fn search_entry(&self) -> gtk::SearchEntry {
        self.imp().search_entry.get()
    }

    #[must_use]
    pub fn search_content(&self) -> adw::Bin {
        self.imp().search_content.get()
    }
}
