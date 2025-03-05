use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use gtk::{gio::prelude::ListModelExt, prelude::WidgetExt};

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/glossary_row.blp")]
    pub struct GlossaryRow {
        #[template_child]
        pub content: TemplateChild<gtk::Box>,
        #[template_child]
        pub tags: TemplateChild<gtk::Box>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for GlossaryRow {
        const NAME: &str = "GlossaryRow";
        type Type = super::GlossaryRow;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for GlossaryRow {
        fn constructed(&self) {
            let content = self.content.get();
            content
                .observe_children()
                .connect_items_changed(move |model, _, _, _| {
                    content.set_visible(model.n_items() > 0);
                });
        }
    }
    impl WidgetImpl for GlossaryRow {}
    impl BinImpl for GlossaryRow {}
}

glib::wrapper! {
    pub struct GlossaryRow(ObjectSubclass<imp::GlossaryRow>) @extends gtk::Widget, adw::Bin;
}

impl GlossaryRow {
    pub fn new() -> Self {
        glib::Object::new()
    }

    pub fn content(&self) -> gtk::Box {
        self.imp().content.get()
    }

    pub fn tags(&self) -> gtk::Box {
        self.imp().tags.get()
    }
}
