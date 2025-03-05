use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/glossary_page.blp")]
    pub struct GlossaryPage;

    #[glib::object_subclass]
    impl ObjectSubclass for GlossaryPage {
        const NAME: &str = "GlossaryPage";
        type Type = super::GlossaryPage;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GlossaryPage {}
    impl WidgetImpl for GlossaryPage {}
    impl BoxImpl for GlossaryPage {}
}

glib::wrapper! {
    pub struct GlossaryPage(ObjectSubclass<imp::GlossaryPage>) @extends gtk::Widget, gtk::Box;
}

impl GlossaryPage {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
