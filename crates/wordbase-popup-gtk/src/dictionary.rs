use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use gtk::prelude::WidgetExt;

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/dictionary.blp")]
    pub struct Dictionary {
        // #[template_child]
        // pub label: TemplateChild<gtk::Label>,
        // #[template_child(id = "my_label2")]
        // pub label2: gtk::TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Dictionary {
        const NAME: &str = "Dictionary";
        type Type = super::Dictionary;
        type ParentType = gtk::Box;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Dictionary {
        fn dispose(&self) {
            while let Some(child) = self.obj().first_child() {
                child.unparent();
            }
        }
    }
    impl WidgetImpl for Dictionary {}
    impl BoxImpl for Dictionary {}
}

glib::wrapper! {
    pub struct Dictionary(ObjectSubclass<imp::Dictionary>) @extends gtk::Widget, gtk::Box;
}

impl Dictionary {
    pub fn new() -> Self {
        glib::Object::new()
    }
}
