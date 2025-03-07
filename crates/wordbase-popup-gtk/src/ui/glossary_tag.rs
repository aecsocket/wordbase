use adw::subclass::prelude::*;
use gtk::glib;
use wordbase::TagCategory;

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

    pub const fn css_class_of(category: TagCategory) -> &'static str {
        match category {
            TagCategory::Name => "name",
            TagCategory::Expression => "expression",
            TagCategory::Popular => "popular",
            TagCategory::Frequent => "frequent",
            TagCategory::Archaism => "archaism",
            TagCategory::Dictionary => "dictionary",
            TagCategory::Frequency => "frequency",
            TagCategory::PartOfSpeech => "part-of-speech",
            TagCategory::Search => "search",
            TagCategory::PronunciationDictionary => "pronunciation-dictionary",
        }
    }
}
