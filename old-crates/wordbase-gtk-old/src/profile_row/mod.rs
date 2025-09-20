use {
    crate::{
        AppEvent, PROFILE, app_window, current_profile, engine, gettext, settings,
        util::{AppComponent, impl_component},
    },
    adw::prelude::*,
    anyhow::{Context as _, Result},
    glib::{SignalHandlerId, clone},
    relm4::prelude::*,
    std::sync::Arc,
    wordbase::{NormString, Profile},
    wordbase_engine::EngineEvent,
};

mod ui;

#[derive(Debug)]
pub struct ProfileRow {
    profile: Arc<Profile>,
    name_changed_handler: SignalHandlerId,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    UpdateUi,
    AskRemove,
    Remove,
    SetName,
}

impl_component!(ProfileRow);

impl AppComponent for ProfileRow {
    type Args = Arc<Profile>;
    type Msg = Msg;
    type Ui = ui::ProfileRow;

    async fn init(
        profile: Self::Args,
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
        let name_changed_handler = ui.name().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::SetName)
        ));

        let settings = settings();
        let profile_id = profile.id;
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

        let model = Self {
            profile,
            name_changed_handler,
        };
        model.update_ui(&ui);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Self::Msg,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        match msg {
            Msg::UpdateUi => {
                self.update_ui(ui);
            }
            Msg::AskRemove => {
                ui.remove_dialog().present(Some(&app_window()));
            }
            Msg::Remove => {
                engine()
                    .remove_profile(self.profile.id)
                    .await
                    .with_context(|| gettext("Failed to remove profile"))?;
            }
            Msg::SetName => {
                let name = NormString::new(ui.name().text());
                ui.name().set_class_active("error", name.is_none());
                let Some(name) = name else {
                    return Ok(());
                };

                engine()
                    .set_profile_name(self.profile.id, Some(name))
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
        match event {
            // AppEvent::Engine(EngineEvent::ProfileNameSet { id }) if id == self.profile.id => {
            //     self.sync(ui);
            // }
            AppEvent::ProfileIdSet => self.sync(ui),
            _ => {}
        }
        Ok(())
    }
}

impl ProfileRow {
    fn sync(&mut self, ui: &ui::ProfileRow) {
        if let Some(profile) = engine().profiles().get(&self.profile.id).cloned() {
            self.profile = profile;
            self.update_ui(ui);
        }
    }

    fn update_ui(&self, ui: &ui::ProfileRow) {
        let name = name_of(&self.profile);
        ui.name().block_signal(&self.name_changed_handler);
        ui.name().set_text(&name);
        ui.name().unblock_signal(&self.name_changed_handler);

        let is_current = current_profile().id == self.profile.id;
        let more_than_1_profile = engine().profiles().len() > 1;
        ui.remove().set_visible(!is_current && more_than_1_profile);
    }
}

pub fn name_of(profile: &Profile) -> String {
    profile.name.as_ref().map_or_else(
        || gettext("Default Profile").into(),
        |s| s.clone().into_inner(),
    )
}
