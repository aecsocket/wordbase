use std::sync::Arc;

use glib::object::Cast;
use gtk::prelude::WidgetExt;
use relm4::adw::{self, glib, gtk, subclass::prelude::*};
use wordbase::Dictionary;

mod imp {
    use std::cell::RefCell;

    use arc_swap::ArcSwapOption;
    use wordbase::Dictionary;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/dictionary_row/ui.blp")]
    pub struct DictionaryRow {
        #[template_child]
        pub imported: TemplateChild<gtk::Box>,
        #[template_child]
        pub enabled: TemplateChild<gtk::CheckButton>,
        #[template_child]
        pub processing: TemplateChild<adw::Bin>,
        #[template_child]
        pub import_error: TemplateChild<gtk::Button>,
        #[template_child]
        pub is_sorting: TemplateChild<gtk::Button>,
        #[template_child]
        pub progress: TemplateChild<gtk::ProgressBar>,
        #[template_child]
        pub action_row: TemplateChild<adw::ActionRow>,
        #[template_child]
        pub set_sorting: TemplateChild<gtk::Button>,
        #[template_child]
        pub visit_website: TemplateChild<gtk::Button>,
        #[template_child]
        pub remove: TemplateChild<gtk::Button>,
        #[template_child]
        pub remove_dialog: TemplateChild<adw::AlertDialog>,
        pub dictionary: ArcSwapOption<Dictionary>,
        pub meta_list: RefCell<Option<gtk::ListBox>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DictionaryRow {
        const NAME: &str = "WdbDictionaryRow";
        type Type = super::DictionaryRow;
        type ParentType = adw::ExpanderRow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DictionaryRow {}
    impl WidgetImpl for DictionaryRow {}
    impl ListBoxRowImpl for DictionaryRow {}
    impl PreferencesRowImpl for DictionaryRow {}
    impl ExpanderRowImpl for DictionaryRow {}
}

glib::wrapper! {
    pub struct DictionaryRow(ObjectSubclass<imp::DictionaryRow>) @extends gtk::Widget, gtk::ListBoxRow, adw::PreferencesRow, adw::ExpanderRow;
}

impl Default for DictionaryRow {
    fn default() -> Self {
        let this = glib::Object::new::<Self>();
        let meta_list = this
            .imp()
            .action_row
            .get()
            .parent()
            .expect("action row should have parent")
            .downcast::<gtk::ListBox>()
            .expect("action row parent should be a list box");
        this.imp().meta_list.replace(Some(meta_list));
        this
    }
}

impl DictionaryRow {
    #[must_use]
    pub fn imported(&self) -> gtk::Box {
        self.imp().imported.get()
    }

    #[must_use]
    pub fn enabled(&self) -> gtk::CheckButton {
        self.imp().enabled.get()
    }

    #[must_use]
    pub fn processing(&self) -> adw::Bin {
        self.imp().processing.get()
    }

    #[must_use]
    pub fn import_error(&self) -> gtk::Button {
        self.imp().import_error.get()
    }

    #[must_use]
    pub fn is_sorting(&self) -> gtk::Button {
        self.imp().is_sorting.get()
    }

    #[must_use]
    pub fn progress(&self) -> gtk::ProgressBar {
        self.imp().progress.get()
    }

    #[must_use]
    pub fn action_row(&self) -> adw::ActionRow {
        self.imp().action_row.get()
    }

    #[must_use]
    pub fn set_sorting(&self) -> gtk::Button {
        self.imp().set_sorting.get()
    }

    #[must_use]
    pub fn visit_website(&self) -> gtk::Button {
        self.imp().visit_website.get()
    }

    #[must_use]
    pub fn remove(&self) -> gtk::Button {
        self.imp().remove.get()
    }

    #[must_use]
    pub fn remove_dialog(&self) -> adw::AlertDialog {
        self.imp().remove_dialog.get()
    }

    #[must_use]
    pub fn dictionary(&self) -> Option<Arc<Dictionary>> {
        self.imp().dictionary.load().as_ref().cloned()
    }

    pub fn set_dictionary(&self, dictionary: Arc<Dictionary>) {
        self.imp().dictionary.store(Some(dictionary));
    }

    #[must_use]
    pub fn meta_list(&self) -> gtk::ListBox {
        self.imp()
            .meta_list
            .borrow()
            .as_ref()
            .cloned()
            .expect("meta list should be initialized")
    }
}
