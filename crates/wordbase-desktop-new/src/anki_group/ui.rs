use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/anki_group/ui.blp")]
    pub struct AnkiGroup {
        #[template_child]
        pub connected: TemplateChild<gtk::Button>,
        #[template_child]
        pub disconnected: TemplateChild<gtk::Button>,

        #[template_child]
        pub deck: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub deck_model: TemplateChild<gtk::StringList>,

        #[template_child]
        pub note_type: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub note_type_model: TemplateChild<gtk::StringList>,

        #[template_child]
        pub note_fields: TemplateChild<adw::ExpanderRow>,
        #[template_child]
        pub field_content_model: TemplateChild<gtk::StringList>,

        #[template_child]
        pub server_url: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub api_key: TemplateChild<adw::EntryRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for AnkiGroup {
        const NAME: &str = "WdbAnkiGroup";
        type Type = super::AnkiGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for AnkiGroup {}
    impl WidgetImpl for AnkiGroup {}
    impl PreferencesGroupImpl for AnkiGroup {}
}

glib::wrapper! {
    pub struct AnkiGroup(ObjectSubclass<imp::AnkiGroup>) @extends gtk::Widget, adw::PreferencesGroup;
}

impl AnkiGroup {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn connected(&self) -> gtk::Button {
        self.imp().connected.get()
    }

    #[must_use]
    pub fn disconnected(&self) -> gtk::Button {
        self.imp().disconnected.get()
    }

    #[must_use]
    pub fn deck(&self) -> adw::ComboRow {
        self.imp().deck.get()
    }

    #[must_use]
    pub fn deck_model(&self) -> gtk::StringList {
        self.imp().deck_model.get()
    }

    #[must_use]
    pub fn note_type(&self) -> adw::ComboRow {
        self.imp().note_type.get()
    }

    #[must_use]
    pub fn note_type_model(&self) -> gtk::StringList {
        self.imp().note_type_model.get()
    }

    #[must_use]
    pub fn note_fields(&self) -> adw::ExpanderRow {
        self.imp().note_fields.get()
    }

    #[must_use]
    pub fn field_content_model(&self) -> gtk::StringList {
        self.imp().field_content_model.get()
    }

    #[must_use]
    pub fn server_url(&self) -> adw::EntryRow {
        self.imp().server_url.get()
    }

    #[must_use]
    pub fn api_key(&self) -> adw::EntryRow {
        self.imp().api_key.get()
    }
}
