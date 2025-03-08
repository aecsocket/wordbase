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

    // https://github.com/yomidevs/yomitan/blob/48f1d012ad5045319d4e492dfbefa39da92817b2/ext/css/display.css#L136-L149
    pub fn css_class_of(category: &str) -> Option<&'static str> {
        match category {
            "name" => Some("name"),
            "expression" => Some("expression"),
            "popular" => Some("popular"),
            "frequent" => Some("frequent"),
            "archaism" => Some("archaism"),
            "dictionary" => Some("dictionary"),
            "frequency" => Some("frequency"),
            "partOfSpeech" => Some("part-of-speech"),
            "search" => Some("search"),
            "pronunciation-dictionary" => Some("pronunciation-dictionary"),
            _ => None,
        }
    }
}
