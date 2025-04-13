use {
    crate::gettext,
    glib::clone,
    gtk4::prelude::ButtonExt,
    relm4::{adw::prelude::*, prelude::*},
};

mod ui;

#[derive(Debug)]
pub struct Model {
    window: gtk::Window,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    AskRemove,
}

#[derive(Debug)]
pub enum Response {
    Remove,
}

impl Component for Model {
    type Init = (gtk::Window, Option<String>);
    type Input = Msg;
    type Output = Response;
    type CommandOutput = ();
    type Root = ui::ThemeRow;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::ThemeRow::new()
    }

    fn init(
        (window, theme_name): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        root.connect_activated(|root| root.enabled().set_active(true));
        root.set_title(
            theme_name
                .as_deref()
                .unwrap_or_else(|| gettext("Default Theme")),
        );
        root.remove_button().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskRemove)
        ));
        root.remove_dialog().connect_response(
            Some("remove_confirm"),
            clone!(
                #[strong]
                sender,
                move |_, _| {
                    _ = sender.output(Response::Remove);
                }
            ),
        );

        if theme_name.is_none() {
            root.remove_button().set_visible(false);
        }

        ComponentParts {
            model: Self { window },
            widgets: (),
        }
    }

    fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            Msg::AskRemove => {
                root.remove_dialog().present(Some(&self.window));
            }
        }
    }
}
