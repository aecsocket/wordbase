use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/manager/ui.blp")]
    pub struct Manager {
        #[template_child]
        pub content_stack: TemplateChild<adw::ViewStack>,
        #[template_child]
        pub page_lookup: TemplateChild<adw::ViewStackPage>,
        #[template_child]
        pub page_landing: TemplateChild<adw::ViewStackPage>,
        #[template_child]
        pub page_no_dictionaries: TemplateChild<adw::ViewStackPage>,

        #[template_child]
        pub settings: TemplateChild<adw::PreferencesPage>,
        #[template_child]
        pub advanced: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub texthooker_url: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub texthooker_connected: TemplateChild<gtk::Widget>,
        #[template_child]
        pub texthooker_disconnected: TemplateChild<gtk::Widget>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub lookup_results: TemplateChild<adw::Bin>,
        #[template_child]
        pub quit: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub profile_menu: TemplateChild<gio::Menu>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Manager {
        const NAME: &str = "WdbManager";
        type Type = super::Manager;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Manager {}
    impl WidgetImpl for Manager {}
    impl BinImpl for Manager {}
}

glib::wrapper! {
    pub struct Manager(ObjectSubclass<imp::Manager>) @extends gtk::Widget, adw::Bin;
}

impl Manager {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn content_stack(&self) -> adw::ViewStack {
        self.imp().content_stack.get()
    }

    #[must_use]
    pub fn page_lookup(&self) -> adw::ViewStackPage {
        self.imp().page_lookup.get()
    }

    #[must_use]
    pub fn page_landing(&self) -> adw::ViewStackPage {
        self.imp().page_landing.get()
    }

    #[must_use]
    pub fn page_no_dictionaries(&self) -> adw::ViewStackPage {
        self.imp().page_no_dictionaries.get()
    }

    #[must_use]
    pub fn profile_menu(&self) -> gio::Menu {
        self.imp().profile_menu.get()
    }

    #[must_use]
    pub fn settings(&self) -> adw::PreferencesPage {
        self.imp().settings.get()
    }

    #[must_use]
    pub fn advanced(&self) -> adw::PreferencesGroup {
        self.imp().advanced.get()
    }

    #[must_use]
    pub fn texthooker_url(&self) -> adw::EntryRow {
        self.imp().texthooker_url.get()
    }

    #[must_use]
    pub fn texthooker_connected(&self) -> gtk::Widget {
        self.imp().texthooker_connected.get()
    }

    #[must_use]
    pub fn texthooker_disconnected(&self) -> gtk::Widget {
        self.imp().texthooker_disconnected.get()
    }

    #[must_use]
    pub fn search_entry(&self) -> gtk::SearchEntry {
        self.imp().search_entry.get()
    }

    #[must_use]
    pub fn lookup_results(&self) -> adw::Bin {
        self.imp().lookup_results.get()
    }

    #[must_use]
    pub fn quit(&self) -> adw::ButtonRow {
        self.imp().quit.get()
    }
}
