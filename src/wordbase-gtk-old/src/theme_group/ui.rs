use relm4::adw::{self, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/theme_group/ui.blp")]
    pub struct ThemeGroup {
        #[template_child]
        pub font_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub font_reset: TemplateChild<gtk::Button>,
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub dummy_group: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub import_button: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub import_dialog: TemplateChild<gtk::FileDialog>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ThemeGroup {
        const NAME: &str = "WdbThemeGroup";
        type Type = super::ThemeGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ThemeGroup {}
    impl WidgetImpl for ThemeGroup {}
    impl PreferencesGroupImpl for ThemeGroup {}
}

glib::wrapper! {
    pub struct ThemeGroup(ObjectSubclass<imp::ThemeGroup>) @extends gtk::Widget, adw::PreferencesGroup;
}

impl Default for ThemeGroup {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ThemeGroup {
    #[must_use]
    pub fn font_row(&self) -> adw::ActionRow {
        self.imp().font_row.get()
    }

    #[must_use]
    pub fn font_reset(&self) -> gtk::Button {
        self.imp().font_reset.get()
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
    pub fn import_button(&self) -> adw::ButtonRow {
        self.imp().import_button.get()
    }

    #[must_use]
    pub fn import_dialog(&self) -> gtk::FileDialog {
        self.imp().import_dialog.get()
    }
}
