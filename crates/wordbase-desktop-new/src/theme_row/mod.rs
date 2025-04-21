use {
    crate::{APP_ID, AppEvent, CUSTOM_THEME, forward_events, gettext, theme::ThemeKey},
    glib::clone,
    relm4::{
        adw::{gio, prelude::*},
        prelude::*,
    },
};

mod ui;

#[derive(Debug)]
pub struct Model {
    key: ThemeKey,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    Select,
    AskRemove,
}

impl AsyncComponent for Model {
    type Init = ThemeKey;
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::ThemeRow;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::ThemeRow::new()
    }

    async fn init(
        key: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);
        let settings = gio::Settings::new(APP_ID);

        let is_enabled = match settings.string(CUSTOM_THEME).as_str() {
            "" => key == ThemeKey::Default,
            s => matches!(key, ThemeKey::Custom(ref name) if &*name.0 == s),
        };
        if is_enabled {
            root.enabled().set_active(true);
        }
        root.connect_activated(|root| root.enabled().set_active(true));
        root.enabled().connect_active_notify(clone!(
            #[strong]
            sender,
            move |active| {
                if active.is_active() {
                    sender.input(Msg::Select);
                }
            }
        ));

        root.set_title(match &key {
            ThemeKey::Default => gettext("Default Theme"),
            ThemeKey::Custom(name) => name,
        });
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

        if matches!(key, ThemeKey::Default) {
            root.remove_button().set_visible(false);
        }

        AsyncComponentParts {
            model: Self {
                window,
                settings,
                key,
            },
            widgets: (),
        }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            Msg::Select => {
                _ = self.settings.set_string(
                    CUSTOM_THEME,
                    match &self.key {
                        ThemeKey::Default => "",
                        ThemeKey::Custom(name) => name,
                    },
                );
                _ = APP_EVENTS.send(AppEvent::ThemeSelected(self.key.clone()));
            }
            Msg::AskRemove => {
                root.remove_dialog().present(Some(&self.window));
            }
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppEvent::ThemeSelected(key) if key == self.key => {
                root.enabled().set_active(true);
            }
            _ => {}
        }
    }
}
