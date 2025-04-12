use relm4::adw::{self, glib, gtk, subclass::prelude::*};

mod imp {
    use super::*;

    #[derive(Debug, Default, gtk::CompositeTemplate)]
    #[template(file = "src/overlay/ui.blp")]
    pub struct Overlay {
        #[template_child]
        pub copy: TemplateChild<gtk::Button>,
        #[template_child]
        pub manager: TemplateChild<gtk::Button>,
        #[template_child]
        pub settings: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub sentence: TemplateChild<gtk::Label>,
        #[template_child]
        pub font_size_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub opacity_idle_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub opacity_hover_scale: TemplateChild<gtk::Scale>,
        #[template_child]
        pub font_size: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub opacity_idle: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub opacity_hover: TemplateChild<gtk::Adjustment>,
        #[template_child]
        pub scan_trigger: TemplateChild<gtk::DropDown>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Overlay {
        const NAME: &str = "WdbOverlay";
        type Type = super::Overlay;
        type ParentType = adw::Window;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Overlay {}
    impl WidgetImpl for Overlay {}
    impl WindowImpl for Overlay {}
    impl AdwWindowImpl for Overlay {}
}

glib::wrapper! {
    pub struct Overlay(ObjectSubclass<imp::Overlay>) @extends gtk::Widget, gtk::Window, adw::Window;
}

impl Overlay {
    #[must_use]
    pub fn new() -> Self {
        glib::Object::new()
    }

    #[must_use]
    pub fn copy(&self) -> gtk::Button {
        self.imp().copy.get()
    }

    #[must_use]
    pub fn manager(&self) -> gtk::Button {
        self.imp().manager.get()
    }

    #[must_use]
    pub fn settings(&self) -> gtk::MenuButton {
        self.imp().settings.get()
    }

    #[must_use]
    pub fn sentence(&self) -> gtk::Label {
        self.imp().sentence.get()
    }

    #[must_use]
    pub fn font_size_scale(&self) -> gtk::Scale {
        self.imp().font_size_scale.get()
    }

    #[must_use]
    pub fn opacity_idle_scale(&self) -> gtk::Scale {
        self.imp().opacity_idle_scale.get()
    }

    #[must_use]
    pub fn opacity_hover_scale(&self) -> gtk::Scale {
        self.imp().opacity_hover_scale.get()
    }

    #[must_use]
    pub fn font_size(&self) -> gtk::Adjustment {
        self.imp().font_size.get()
    }

    #[must_use]
    pub fn opacity_idle(&self) -> gtk::Adjustment {
        self.imp().opacity_idle.get()
    }

    #[must_use]
    pub fn opacity_hover(&self) -> gtk::Adjustment {
        self.imp().opacity_hover.get()
    }

    #[must_use]
    pub fn scan_trigger(&self) -> gtk::DropDown {
        self.imp().scan_trigger.get()
    }
}
