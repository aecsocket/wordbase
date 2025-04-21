use {
    crate::{
        AppEvent, CURRENT_PROFILE_ID, PROFILE, SignalHandler, app_window, engine, gettext,
        settings,
        util::{AppComponent, impl_component},
    },
    adw::prelude::*,
    anyhow::{Context as _, Result},
    glib::clone,
    relm4::prelude::*,
    wordbase::{NormString, ProfileId},
    wordbase_engine::{EngineEvent, ProfileEvent},
};

mod ui;

#[derive(Debug)]
pub struct ProfileRow {
    profile_id: ProfileId,
    _profile_changed_handler: SignalHandler,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    UpdateMeta,
    AskRemove,
    Remove,
    SetName,
}

impl_component!(ProfileRow);

impl AppComponent for ProfileRow {
    type Args = ProfileId;
    type Msg = Msg;
    type Ui = ui::ProfileRow;

    async fn init(
        profile_id: Self::Args,
        ui: Self::Ui,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        ui.remove().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskRemove)
        ));
        ui.remove_dialog().connect_response(
            Some("remove_confirm"),
            clone!(
                #[strong]
                sender,
                move |_, _| sender.input(Msg::Remove)
            ),
        );
        ui.name().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SetName)
        ));

        let settings = settings();
        settings
            .bind(PROFILE, &ui.current(), "active")
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
        let profile_changed_handler = SignalHandler::new(&settings, |it| {
            it.connect_changed(
                Some(PROFILE),
                clone!(
                    #[strong]
                    sender,
                    move |_, _| sender.input(Msg::UpdateMeta)
                ),
            )
        });

        let model = Self {
            profile_id,
            _profile_changed_handler: profile_changed_handler,
        };
        model.update_meta(&ui);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Self::Msg,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        match msg {
            Msg::UpdateMeta => {
                self.update_meta(ui);
            }
            Msg::AskRemove => {
                ui.remove_dialog().present(Some(&app_window()));
            }
            Msg::Remove => {
                engine()
                    .remove_profile(self.profile_id)
                    .await
                    .with_context(|| gettext("Failed to remove profile"))?;
            }
            Msg::SetName => {
                let name = NormString::new(ui.name().text());
                let Some(name) = name else {
                    ui.name().set_css_classes(&["error"]);
                    return Ok(());
                };

                ui.name().set_css_classes(&[]);
                let Some(profile) = engine().profiles().get(&self.profile_id).cloned() else {
                    return Ok(());
                };

                let mut config = profile.config.clone();
                config.name = Some(name);
                engine()
                    .set_profile_config(self.profile_id, config)
                    .await
                    .with_context(|| gettext("Failed to set profile name"))?;
            }
        }
        Ok(())
    }

    async fn update_event(
        &mut self,
        event: AppEvent,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        if let AppEvent::Engine(EngineEvent::Profile(
            ProfileEvent::Added(_) | ProfileEvent::Removed(_),
        )) = event
        {
            self.update_meta(ui);
        }
        Ok(())
    }
}

impl ProfileRow {
    fn update_meta(&self, ui: &ui::ProfileRow) {
        let Some(profile) = engine().profiles().get(&self.profile_id).cloned() else {
            return;
        };

        let name = profile
            .config
            .name
            .as_ref()
            .map_or_else(|| gettext("Default Profile"), |s| s.as_str());
        ui.name().set_text(name);

        let is_current = *CURRENT_PROFILE_ID.read() == self.profile_id;
        let more_than_1_profile = engine().profiles().len() > 1;
        ui.remove().set_visible(!is_current && more_than_1_profile);
    }
}
