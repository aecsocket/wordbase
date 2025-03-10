use {
    adw::subclass::prelude::*,
    gtk::glib,
    wordbase::{RecordKind, protocol::LookupResponse},
};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/dictionary.blp")]
    pub struct Dictionary;

    #[glib::object_subclass]
    impl ObjectSubclass for Dictionary {
        const NAME: &str = "Dictionary";
        type Type = super::Dictionary;
        type ParentType = gtk::Grid;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Dictionary {}
    impl WidgetImpl for Dictionary {}
    impl GridImpl for Dictionary {}
}

glib::wrapper! {
    pub struct Dictionary(ObjectSubclass<imp::Dictionary>) @extends gtk::Widget, gtk::Grid;
}

impl Dictionary {
    pub const SUPPORTED_RECORD_KINDS: &[RecordKind] = &[
        // meta
        RecordKind::Frequency,
        RecordKind::JpPitch,
        // glossaries
        RecordKind::GlossaryPlainText,
        RecordKind::GlossaryHtml,
        RecordKind::YomitanGlossary,
    ];

    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn from<'a>(records: impl IntoIterator<Item = &'a LookupResponse>) -> Self {
        let this = Self::new();
        this
    }
}
