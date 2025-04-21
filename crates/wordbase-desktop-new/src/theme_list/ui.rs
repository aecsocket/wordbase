use relm4::adw::{self, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/manager/theme_list/ui.blp")]
    pub struct ThemeList {
        #[template_child]
        pub font_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub font_reset: TemplateChild<gtk::Button>,
        #[template_child]
        pub enabled_dummy: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub import_button: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub import_dialog: TemplateChild<gtk::FileDialog>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ThemeList {
        const NAME: &str = "WdbThemeList";
        type Type = super::ThemeList;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ThemeList {}
    impl WidgetImpl for ThemeList {}
    impl BinImpl for ThemeList {}
}

glib::wrapper! {
    pub struct ThemeList(ObjectSubclass<imp::ThemeList>) @extends gtk::Widget, adw::Bin;
}

impl ThemeList {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn font_row(&self) -> adw::ActionRow {
        self.imp().font_row.get()
    }

    #[must_use]
    pub fn font_reset(&self) -> gtk::Button {
        self.imp().font_reset.get()
    }

    #[must_use]
    pub fn enabled_dummy(&self) -> gtk::CheckButton {
        self.imp().enabled_dummy.get()
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
