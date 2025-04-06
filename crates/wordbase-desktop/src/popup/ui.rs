use relm4::adw::{self, gio, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/popup/ui.blp")]
    pub struct Popup {
        #[template_child]
        pub content: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub manager_profiles: TemplateChild<adw::SplitButton>,
        #[template_child]
        pub profiles_menu: TemplateChild<gio::Menu>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Popup {
        const NAME: &str = "Popup";
        type Type = super::Popup;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Popup {}
    impl WidgetImpl for Popup {}
    impl WindowImpl for Popup {}
    impl AdwWindowImpl for Popup {}
}

glib::wrapper! {
    pub struct Popup(ObjectSubclass<imp::Popup>) @extends gtk::Widget, gtk::Window, adw::Window;
}

impl Popup {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn content(&self) -> gtk::Overlay {
        self.imp().content.get()
    }

    #[must_use]
    pub fn manager_profiles(&self) -> adw::SplitButton {
        self.imp().manager_profiles.get()
    }

    #[must_use]
    pub fn profiles_menu(&self) -> gio::Menu {
        self.imp().profiles_menu.get()
    }
}
