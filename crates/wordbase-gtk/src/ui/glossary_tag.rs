use {adw::subclass::prelude::*, gtk::glib};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/glossary_tag.blp")]
    pub struct GlossaryTag;

    #[glib::object_subclass]
    impl ObjectSubclass for GlossaryTag {
        const NAME: &str = "GlossaryTag";
        type Type = super::GlossaryTag;
        type ParentType = gtk::Button;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GlossaryTag {}
    impl WidgetImpl for GlossaryTag {}
    impl ButtonImpl for GlossaryTag {}
}

glib::wrapper! {
    pub struct GlossaryTag(ObjectSubclass<imp::GlossaryTag>) @extends gtk::Widget, gtk::Button;
}

impl GlossaryTag {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
