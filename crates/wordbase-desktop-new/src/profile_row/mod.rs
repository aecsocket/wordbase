use adw::prelude::*;
use anyhow::Context as _;
use glib::clone;
use relm4::prelude::*;
use wordbase::{NormString, ProfileId};
use wordbase_engine::{EngineEvent, ProfileEvent};

use crate::{
    AppEvent, CURRENT_PROFILE_ID, PROFILE, engine, forward_events, gettext, handle_result, settings,
};

mod ui;

#[derive(Debug)]
pub struct ProfileRow {
    window: gtk::Window,
    profile_id: ProfileId,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    UpdateMeta,
    AskRemove,
    Remove,
    SetName,
}

impl AsyncComponent for ProfileRow {
    type Init = (gtk::Window, ProfileId);
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::ProfileRow;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::ProfileRow::new()
    }

    async fn init(
        (window, profile_id): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);

        root.remove().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskRemove)
        ));
        root.remove_dialog().connect_response(
            Some("remove_confirm"),
            clone!(
                #[strong]
                sender,
                move |_, _| sender.input(Msg::Remove)
            ),
        );
        root.name().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SetName)
        ));

        let settings = settings();
        settings
            .bind(PROFILE, &root.current(), "active")
            .mapping(move |setting, _| {
                Some((setting.str() == Some(&format!("{}", profile_id.0))).to_value())
            })
            .set_mapping(move |value, _| {
                if value.get::<bool>().expect("`active` should be a bool") {
                    Some(format!("{}", profile_id.0).to_variant())
                } else {
                    None
                }
            })
            .build();
        settings.connect_changed(
            Some(PROFILE),
            clone!(
                #[strong]
                sender,
                move |_, _| sender.input(Msg::UpdateMeta)
            ),
        );

        let model = Self { window, profile_id };
        model.update_meta(&root);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            Msg::UpdateMeta => {
                self.update_meta(root);
            }
            Msg::AskRemove => {
                root.remove_dialog().present(Some(&self.window));
            }
            Msg::Remove => {
                handle_result(
                    engine()
                        .remove_profile(self.profile_id)
                        .await
                        .with_context(|| gettext("Failed to remove profile")),
                );
            }
            Msg::SetName => {
                let name = NormString::new(root.name().text());
                let Some(name) = name else {
                    root.name().set_css_classes(&["error"]);
                    return;
                };

                root.name().set_css_classes(&[]);
                let Some(profile) = engine().profiles().get(&self.profile_id).cloned() else {
                    return;
                };

                let mut config = profile.config.clone();
                config.name = Some(name);
                handle_result(
                    engine()
                        .set_profile_config(self.profile_id, config)
                        .await
                        .with_context(|| gettext("Failed to set profile name")),
                );
            }
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        event: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        if let AppEvent::Engine(EngineEvent::Profile(
            ProfileEvent::Added(_) | ProfileEvent::Removed(_),
        )) = event
        {
            self.update_meta(root);
        }
    }
}

impl ProfileRow {
    fn update_meta(&self, root: &ui::ProfileRow) {
        let Some(profile) = engine().profiles().get(&self.profile_id).cloned() else {
            return;
        };

        let name = profile
            .config
            .name
            .as_ref()
            .map_or_else(|| gettext("Default Profile"), |s| s.as_str());
        root.name().set_text(name);

        let is_current = *CURRENT_PROFILE_ID.read() == self.profile_id;
        let more_than_1_profile = engine().profiles().len() > 1;
        root.remove()
            .set_visible(!is_current && more_than_1_profile);
    }
}
