use relm4::adw::{self, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/manager/theme_list/ui.blp")]
    pub struct Themes {
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub import_button: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub import_dialog: TemplateChild<gtk::FileDialog>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Themes {
        const NAME: &str = "WdbThemes";
        type Type = super::Themes;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Themes {}
    impl WidgetImpl for Themes {}
    impl BinImpl for Themes {}
}

glib::wrapper! {
    pub struct Themes(ObjectSubclass<imp::Themes>) @extends gtk::Widget, adw::Bin;
}

impl Themes {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn list(&self) -> gtk::ListBox {
        self.imp().list.get()
    }

    #[must_use]
    pub fn import_button(&self) -> adw::ButtonRow {
        self.imp().import_button.get()
    }

    #[must_use]
    pub fn import_dialog(&self) -> gtk::FileDialog {
        self.imp().import_dialog.get()
    }
}
