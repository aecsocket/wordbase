use adw::subclass::prelude::*;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/profile_row/ui.blp")]
    pub struct ProfileRow {
        #[template_child]
        pub current: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub name: TemplateChild<gtk::Entry>,
        #[template_child]
        pub remove: TemplateChild<gtk::Button>,
        #[template_child]
        pub remove_dialog: TemplateChild<adw::AlertDialog>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProfileRow {
        const NAME: &str = "WdbProfileRow";
        type Type = super::ProfileRow;
        type ParentType = adw::PreferencesRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ProfileRow {}
    impl WidgetImpl for ProfileRow {}
    impl ListBoxRowImpl for ProfileRow {}
    impl PreferencesRowImpl for ProfileRow {}
}

glib::wrapper! {
    pub struct ProfileRow(ObjectSubclass<imp::ProfileRow>) @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow;
}

impl Default for ProfileRow {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl ProfileRow {
    #[must_use]
    pub fn current(&self) -> gtk::CheckButton {
        self.imp().current.get()
    }

    #[must_use]
    pub fn name(&self) -> gtk::Entry {
        self.imp().name.get()
    }

    #[must_use]
    pub fn remove(&self) -> gtk::Button {
        self.imp().remove.get()
    }

    #[must_use]
    pub fn remove_dialog(&self) -> adw::AlertDialog {
        self.imp().remove_dialog.get()
    }
}
