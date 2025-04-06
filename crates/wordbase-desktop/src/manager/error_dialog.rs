use relm4::adw::{self, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/manager/error_dialog.blp")]
    pub struct ErrorDialog {
        #[template_child]
        pub message: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ErrorDialog {
        const NAME: &str = "ErrorDialog";
        type Type = super::ErrorDialog;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for ErrorDialog {}
    impl WidgetImpl for ErrorDialog {}
    impl BinImpl for ErrorDialog {}
}

glib::wrapper! {
    pub struct ErrorDialog(ObjectSubclass<imp::ErrorDialog>) @extends gtk::Widget, adw::Bin;
}

impl ErrorDialog {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn message(&self) -> gtk::Label {
        self.imp().message.get()
    }
}
