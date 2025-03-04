use adw::{prelude::BinExt, subclass::prelude::*};
use gtk::glib;

use super::Dictionary;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/dictionary_popup.blp")]
    pub struct DictionaryPopup {
        #[template_child]
        pub dictionary: TemplateChild<adw::Bin>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DictionaryPopup {
        const NAME: &str = "DictionaryPopup";
        type Type = super::DictionaryPopup;
        type ParentType = adw::Bin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DictionaryPopup {}
    impl WidgetImpl for DictionaryPopup {}
    impl BinImpl for DictionaryPopup {}
}

glib::wrapper! {
    pub struct DictionaryPopup(ObjectSubclass<imp::DictionaryPopup>) @extends gtk::Widget, adw::Bin;
}

impl DictionaryPopup {
    pub fn new(dictionary: &Dictionary) -> Self {
        let this = glib::Object::new::<Self>();
        this.imp().dictionary.set_child(Some(dictionary));
        this
    }
}
