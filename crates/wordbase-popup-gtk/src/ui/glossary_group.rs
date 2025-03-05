use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/glossary_group.blp")]
    pub struct GlossaryGroup {
        #[template_child]
        pub source: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GlossaryGroup {
        const NAME: &str = "GlossaryGroup";
        type Type = super::GlossaryGroup;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GlossaryGroup {}
    impl WidgetImpl for GlossaryGroup {}
    impl BoxImpl for GlossaryGroup {}
}

glib::wrapper! {
    pub struct GlossaryGroup(ObjectSubclass<imp::GlossaryGroup>) @extends gtk::Widget, gtk::Box;
}

impl GlossaryGroup {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn source(&self) -> gtk::Label {
        self.imp().source.get()
    }
}
