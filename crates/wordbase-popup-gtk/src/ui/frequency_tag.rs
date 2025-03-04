use adw::subclass::prelude::*;
use gtk::glib;
use wordbase::dict;

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/ui/frequency_tag.blp")]
    pub struct FrequencyTag {
        #[template_child]
        pub dictionary: TemplateChild<gtk::Label>,
        #[template_child]
        pub frequency: TemplateChild<gtk::Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for FrequencyTag {
        const NAME: &str = "FrequencyTag";
        type Type = super::FrequencyTag;
        type ParentType = gtk::Button;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for FrequencyTag {}
    impl WidgetImpl for FrequencyTag {}
    impl ButtonImpl for FrequencyTag {}
}

glib::wrapper! {
    pub struct FrequencyTag(ObjectSubclass<imp::FrequencyTag>) @extends gtk::Widget, gtk::Button;
}

impl FrequencyTag {
    #[must_use]
    pub fn new(dictionary: &str, frequency: &str) -> Self {
        let this = glib::Object::new::<Self>();
        this.imp().dictionary.set_text(dictionary);
        this.imp().frequency.set_text(frequency);
        this
    }

    #[must_use]
    pub fn from(value: &dict::FrequencySet) -> Self {
        let display_frequency = value
            .frequencies
            .iter()
            .map(|frequency| {
                frequency
                    .display_value
                    .as_ref()
                    .map_or_else(|| format!("{}", frequency.value), ToOwned::to_owned)
            })
            .collect::<Vec<_>>()
            .join(" · ");

        Self::new(&value.dictionary, &display_frequency)
    }

    #[must_use]
    pub fn dictionary(&self) -> glib::GString {
        self.imp().dictionary.text()
    }

    pub fn set_dictionary(&self, dictionary: &str) {
        self.imp().dictionary.set_text(dictionary);
    }

    #[must_use]
    pub fn frequency(&self) -> glib::GString {
        self.imp().frequency.text()
    }

    pub fn set_frequency(&self, frequency: &str) {
        self.imp().frequency.set_text(frequency);
    }
}
