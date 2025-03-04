use adw::subclass::prelude::*;
use gtk::{glib, prelude::GridExt};
use wordbase::dict;

use super::{EntryMeta, Glossaries};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/dictionary.blp")]
    pub struct Dictionary {
        #[template_child]
        pub grid: TemplateChild<gtk::Grid>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Dictionary {
        const NAME: &str = "Dictionary";
        type Type = super::Dictionary;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Dictionary {}
    impl WidgetImpl for Dictionary {}
    impl BoxImpl for Dictionary {}
}

glib::wrapper! {
    pub struct Dictionary(ObjectSubclass<imp::Dictionary>) @extends gtk::Widget, gtk::Box;
}

impl Dictionary {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn from<'a>(expressions: impl IntoIterator<Item = &'a dict::ExpressionEntry>) -> Self {
        let this = Self::new();
        for (row, expression) in expressions.into_iter().enumerate() {
            let Ok(row) = i32::try_from(row) else {
                break;
            };
            this.attach_entry(row, expression);
        }
        this
    }

    pub fn attach(&self, row: i32, entry_meta: &EntryMeta, glossaries: &Glossaries) {
        self.imp().grid.attach(entry_meta, 0, row, 1, 1);
        self.imp().grid.attach(glossaries, 1, row, 1, 1);
    }

    pub fn attach_entry(&self, row: i32, entry: &dict::ExpressionEntry) {
        let entry_meta = EntryMeta::from(entry);
        let glossaries = Glossaries::from(&entry.glossary_sets);
        self.attach(row, &entry_meta, &glossaries);
    }
}
