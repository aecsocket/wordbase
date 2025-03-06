use adw::subclass::prelude::*;
use gtk::glib;

mod imp {
    use std::cell::RefCell;

    use gtk::{
        gio::{self, prelude::ListModelExt},
        prelude::WidgetExt,
    };

    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/glossary_row.blp")]
    pub struct GlossaryRow {
        #[template_child]
        pub content: TemplateChild<gtk::Box>,
        #[template_child]
        pub tags: TemplateChild<gtk::Box>,
        pub tag_children: RefCell<Option<gio::ListModel>>,
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

    fn hide_if_empty(content: &gtk::Box, model: &gio::ListModel) {
        content.set_visible(model.n_items() > 0);
    }

    impl ObjectImpl for GlossaryRow {
        fn constructed(&self) {
            let tags = self.tags.get();
            let tag_children = tags.observe_children();
            hide_if_empty(&tags, &tag_children);
            tag_children.connect_items_changed(move |child_model, _, _, _| {
                hide_if_empty(&tags, child_model);
            });
            // we must retain a reference to the list model,
            // otherwise we won't receive any signals on it
            self.tag_children.replace(Some(tag_children));
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
