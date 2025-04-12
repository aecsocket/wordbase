use relm4::adw::{self, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/manager/ui.blp")]
    pub struct Manager {
        #[template_child]
        pub toast_overlay: TemplateChild<adw::ToastOverlay>,
        #[template_child]
        pub current_profile: TemplateChild<gtk::DropDown>,
        #[template_child]
        pub profiles_model: TemplateChild<gtk::StringList>,
        #[template_child]
        pub dictionaries: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub themes: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub ankiconnect_server_url: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub ankiconnect_connected: TemplateChild<gtk::Button>,
        #[template_child]
        pub ankiconnect_disconnected: TemplateChild<gtk::Button>,
        #[template_child]
        pub ankiconnect_api_key: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub texthooker_url: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub texthooker_connected: TemplateChild<gtk::Widget>,
        #[template_child]
        pub texthooker_disconnected: TemplateChild<gtk::Widget>,
        #[template_child]
        pub search_entry: TemplateChild<gtk::SearchEntry>,
        #[template_child]
        pub search_sidebar_toggle: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub search_view: TemplateChild<adw::OverlaySplitView>,
        #[template_child]
        pub search_dictionaries: TemplateChild<adw::Bin>,
        #[template_child]
        pub search_themes: TemplateChild<adw::Bin>,
        #[template_child]
        pub quit: TemplateChild<adw::ButtonRow>,
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
    pub fn toast_overlay(&self) -> adw::ToastOverlay {
        self.imp().toast_overlay.get()
    }

    #[must_use]
    pub fn current_profile(&self) -> gtk::DropDown {
        self.imp().current_profile.get()
    }

    #[must_use]
    pub fn profiles_model(&self) -> gtk::StringList {
        self.imp().profiles_model.get()
    }

    #[must_use]
    pub fn dictionaries(&self) -> adw::PreferencesGroup {
        self.imp().dictionaries.get()
    }

    #[must_use]
    pub fn themes(&self) -> adw::PreferencesGroup {
        self.imp().themes.get()
    }

    #[must_use]
    pub fn ankiconnect_server_url(&self) -> adw::EntryRow {
        self.imp().ankiconnect_server_url.get()
    }

    #[must_use]
    pub fn ankiconnect_connected(&self) -> gtk::Button {
        self.imp().ankiconnect_connected.get()
    }

    #[must_use]
    pub fn ankiconnect_disconnected(&self) -> gtk::Button {
        self.imp().ankiconnect_disconnected.get()
    }

    #[must_use]
    pub fn ankiconnect_api_key(&self) -> adw::EntryRow {
        self.imp().ankiconnect_api_key.get()
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
    pub fn search_sidebar_toggle(&self) -> gtk::ToggleButton {
        self.imp().search_sidebar_toggle.get()
    }

    #[must_use]
    pub fn search_view(&self) -> adw::OverlaySplitView {
        self.imp().search_view.get()
    }

    #[must_use]
    pub fn search_dictionaries(&self) -> adw::Bin {
        self.imp().search_dictionaries.get()
    }

    #[must_use]
    pub fn search_themes(&self) -> adw::Bin {
        self.imp().search_themes.get()
    }

    #[must_use]
    pub fn quit(&self) -> adw::ButtonRow {
        self.imp().quit.get()
    }
}
