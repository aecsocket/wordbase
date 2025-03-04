use adw::subclass::prelude::*;
use gtk::{glib, prelude::BoxExt};
use wordbase::dict;

use super::FrequencyTag;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/entry_meta.blp")]
    pub struct EntryMeta {
        #[template_child]
        pub reading: TemplateChild<gtk::Label>,
        #[template_child]
        pub expression: TemplateChild<gtk::Label>,
        #[template_child]
        pub frequency_tags: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for EntryMeta {
        const NAME: &str = "EntryMeta";
        type Type = super::EntryMeta;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for EntryMeta {}
    impl WidgetImpl for EntryMeta {}
    impl BoxImpl for EntryMeta {}
}

glib::wrapper! {
    pub struct EntryMeta(ObjectSubclass<imp::EntryMeta>) @extends gtk::Widget, gtk::Box;
}

impl EntryMeta {
    #[must_use]
    pub fn new(expression: &str, reading: &str) -> Self {
        let this = glib::Object::new::<Self>();
        this.imp().expression.set_text(expression);
        this.imp().reading.set_text(reading);
        this
    }

    #[must_use]
    pub fn from(entry: &dict::ExpressionEntry) -> Self {
        let this = Self::new(entry.reading.expression(), entry.reading.reading());
        for frequency_set in &entry.frequency_sets {
            this.add_frequency_tag(&FrequencyTag::from(frequency_set));
        }
        this
    }

    pub fn add_frequency_tag(&self, frequency_tag: &FrequencyTag) {
        self.imp().frequency_tags.append(frequency_tag);
    }
}
