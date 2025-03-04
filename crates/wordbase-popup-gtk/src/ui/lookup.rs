use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/lookup.blp")]
    pub struct Lookup {
        #[template_child]
        pub entry: TemplateChild<gtk::Entry>,
        #[template_child]
        pub lemma: TemplateChild<gtk::Label>,
        #[template_child]
        pub dictionary_container: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Lookup {
        const NAME: &str = "Lookup";
        type Type = super::Lookup;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Lookup {}
    impl WidgetImpl for Lookup {}
    impl BinImpl for Lookup {}
}

glib::wrapper! {
    pub struct Lookup(ObjectSubclass<imp::Lookup>) @extends gtk::Widget, adw::Bin;
}

impl Lookup {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn entry(&self) -> gtk::Entry {
        self.imp().entry.get()
    }

    #[must_use]
    pub fn lemma(&self) -> gtk::Label {
        self.imp().lemma.get()
    }

    #[must_use]
    pub fn dictionary_container(&self) -> adw::Bin {
        self.imp().dictionary_container.get()
    }
}
