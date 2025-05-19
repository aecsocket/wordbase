use relm4::adw::{self, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/dictionary_group/ui.blp")]
    pub struct DictionaryGroup {
        #[template_child]
        pub list: TemplateChild<gtk::ListBox>,
        #[template_child]
        pub import_button: TemplateChild<adw::ButtonRow>,
        #[template_child]
        pub import_dialog: TemplateChild<gtk::FileDialog>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DictionaryGroup {
        const NAME: &str = "WdbDictionaryGroup";
        type Type = super::DictionaryGroup;
        type ParentType = adw::PreferencesGroup;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DictionaryGroup {}
    impl WidgetImpl for DictionaryGroup {}
    impl PreferencesGroupImpl for DictionaryGroup {}
}

glib::wrapper! {
    pub struct DictionaryGroup(ObjectSubclass<imp::DictionaryGroup>) @extends gtk::Widget, adw::PreferencesGroup;
}

impl Default for DictionaryGroup {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl DictionaryGroup {
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
