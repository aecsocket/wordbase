use relm4::adw::{self, gio, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/overlay/ui.blp")]
    pub struct Overlay {
        #[template_child]
        pub content: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub settings: TemplateChild<gtk::Button>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Overlay {
        const NAME: &str = "Overlay";
        type Type = super::Overlay;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Overlay {}
    impl WidgetImpl for Overlay {}
    impl WindowImpl for Overlay {}
    impl AdwWindowImpl for Overlay {}
}

glib::wrapper! {
    pub struct Overlay(ObjectSubclass<imp::Overlay>) @extends gtk::Widget, gtk::Window, adw::Window;
}

impl Overlay {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn content(&self) -> gtk::Overlay {
        self.imp().content.get()
    }

    #[must_use]
    pub fn settings(&self) -> gtk::Button {
        self.imp().settings.get()
    }
}
