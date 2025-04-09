use relm4::adw::{self, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/manager/theme_row/ui.blp")]
    pub struct ThemeRow {
        #[template_child]
        pub enabled: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub remove_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub remove_dialog: TemplateChild<adw::AlertDialog>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ThemeRow {
        const NAME: &str = "WdbThemeRow";
        type Type = super::ThemeRow;
        type ParentType = adw::ActionRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ThemeRow {}
    impl WidgetImpl for ThemeRow {}
    impl ListBoxRowImpl for ThemeRow {}
    impl PreferencesRowImpl for ThemeRow {}
    impl ActionRowImpl for ThemeRow {}
}

glib::wrapper! {
    pub struct ThemeRow(ObjectSubclass<imp::ThemeRow>) @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ActionRow;
}

impl ThemeRow {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn enabled(&self) -> gtk::CheckButton {
        self.imp().enabled.get()
    }

    #[must_use]
    pub fn remove_button(&self) -> gtk::Button {
        self.imp().remove_button.get()
    }

    #[must_use]
    pub fn remove_dialog(&self) -> adw::AlertDialog {
        self.imp().remove_dialog.get()
    }
}
