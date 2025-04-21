use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/manage_profiles/ui.blp")]
    pub struct ManageProfiles {
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub dummy_group: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub add_profile_name: TemplateChild<adw::EntryRow>,
        #[template_child]
        pub add_profile: TemplateChild<adw::ButtonRow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ManageProfiles {
        const NAME: &str = "WdbManageProfiles";
        type Type = super::ManageProfiles;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ManageProfiles {}
    impl WidgetImpl for ManageProfiles {}
    impl BinImpl for ManageProfiles {}
}

glib::wrapper! {
    pub struct ManageProfiles(ObjectSubclass<imp::ManageProfiles>) @extends gtk::Widget, adw::Bin;
}

impl Default for ManageProfiles {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ManageProfiles {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn list(&self) -> gtk::ListBox {
        self.imp().list.get()
    }

    #[must_use]
    pub fn dummy_group(&self) -> gtk::CheckButton {
        self.imp().dummy_group.get()
    }

    #[must_use]
    pub fn add_profile_name(&self) -> adw::EntryRow {
        self.imp().add_profile_name.get()
    }

    #[must_use]
    pub fn add_profile(&self) -> adw::ButtonRow {
        self.imp().add_profile.get()
    }
}
