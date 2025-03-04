use adw::subclass::prelude::*;
use gtk::{glib, prelude::BoxExt};
use wordbase::dict;

use super::GlossarySet;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/glossaries.blp")]
    pub struct Glossaries;

    #[glib::object_subclass]
    impl ObjectSubclass for Glossaries {
        const NAME: &str = "Glossaries";
        type Type = super::Glossaries;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Glossaries {}
    impl WidgetImpl for Glossaries {}
    impl BoxImpl for Glossaries {}
}

glib::wrapper! {
    pub struct Glossaries(ObjectSubclass<imp::Glossaries>) @extends gtk::Widget, gtk::Box;
}

impl Glossaries {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn from<'a>(glossary_sets: impl IntoIterator<Item = &'a dict::GlossarySet>) -> Self {
        let this = Self::new();
        for glossary_set in glossary_sets {
            this.append(&GlossarySet::from(glossary_set));
        }
        this
    }
}
