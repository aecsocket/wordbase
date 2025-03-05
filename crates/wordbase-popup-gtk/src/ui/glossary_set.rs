use adw::subclass::prelude::*;
use gtk::glib;
use wordbase::schema;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/glossary_set.blp")]
    pub struct GlossarySet {
        #[template_child]
        pub dictionary: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GlossarySet {
        const NAME: &str = "GlossarySet";
        type Type = super::GlossarySet;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GlossarySet {}
    impl WidgetImpl for GlossarySet {}
    impl BinImpl for GlossarySet {}
}

glib::wrapper! {
    pub struct GlossarySet(ObjectSubclass<imp::GlossarySet>) @extends gtk::Widget, adw::Bin;
}

impl GlossarySet {
    pub fn new(dictionary: &str) -> Self {
        let this = glib::Object::new::<Self>();
        this.imp().dictionary.set_text(dictionary);
        this
    }

    pub fn from(set: &schema::GlossarySet) -> Self {
        let this = Self::new(&set.dictionary);
        for glossary in &set.glossaries {}
        this
    }
}
