use adw::{glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/settings.blp")]
    pub struct Settings;

    #[glib::object_subclass]
    impl ObjectSubclass for Settings {
        const NAME: &str = "Settings";
        type Type = super::Settings;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Settings {}
    impl WidgetImpl for Settings {}
    impl BinImpl for Settings {}
}

glib::wrapper! {
    pub struct Settings(ObjectSubclass<imp::Settings>) @extends gtk::Widget, adw::Bin;
}

impl Settings {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
