use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/term_meta.blp")]
    pub struct TermMeta {
        #[template_child]
        pub reading: TemplateChild<gtk::Label>,
        #[template_child]
        pub expression: TemplateChild<gtk::Label>,
        #[template_child]
        pub pitches: TemplateChild<gtk::Box>,
        #[template_child]
        pub frequency_tags: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TermMeta {
        const NAME: &str = "TermMeta";
        type Type = super::TermMeta;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for TermMeta {}
    impl WidgetImpl for TermMeta {}
    impl BoxImpl for TermMeta {}
}

glib::wrapper! {
    pub struct TermMeta(ObjectSubclass<imp::TermMeta>) @extends gtk::Widget, gtk::Box;
}

impl TermMeta {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn reading(&self) -> gtk::Label {
        self.imp().reading.get()
    }

    #[must_use]
    pub fn expression(&self) -> gtk::Label {
        self.imp().expression.get()
    }

    #[must_use]
    pub fn pitches(&self) -> gtk::Box {
        self.imp().pitches.get()
    }

    #[must_use]
    pub fn frequency_tags(&self) -> gtk::Box {
        self.imp().frequency_tags.get()
    }
}
