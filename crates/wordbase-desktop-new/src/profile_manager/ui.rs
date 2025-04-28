use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/profile_manager/ui.blp")]
    pub struct ProfileManager {
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub create: TemplateChild<gtk::Button>,
        #[template_child]
        pub dummy_group: TemplateChild<gtk::CheckButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProfileManager {
        const NAME: &str = "WdbProfileManager";
        type Type = super::ProfileManager;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProfileManager {}
    impl WidgetImpl for ProfileManager {}
    impl BinImpl for ProfileManager {}
}

glib::wrapper! {
    pub struct ProfileManager(ObjectSubclass<imp::ProfileManager>) @extends gtk::Widget, adw::Bin;
}

impl Default for ProfileManager {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ProfileManager {
    #[must_use]
    pub fn list(&self) -> gtk::ListBox {
        self.imp().list.get()
    }

    #[must_use]
    pub fn create(&self) -> gtk::Button {
        self.imp().create.get()
    }

    #[must_use]
    pub fn dummy_group(&self) -> gtk::CheckButton {
        self.imp().dummy_group.get()
    }
}
