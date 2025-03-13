use adw::{glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/overlay/ui.blp")]
    pub struct Overlay {
        #[template_child]
        pub sentence: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Overlay {
        const NAME: &str = "Overlay";
        type Type = super::Overlay;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Overlay {}
    impl WidgetImpl for Overlay {}
    impl BinImpl for Overlay {}
}

glib::wrapper! {
    pub struct Overlay(ObjectSubclass<imp::Overlay>) @extends gtk::Widget, adw::Bin;
}

impl Overlay {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn sentence(&self) -> gtk::Label {
        self.imp().sentence.get()
    }
}
