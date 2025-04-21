use {
    crate::{
        AppEvent, current_profile_id, engine, gettext,
        profile_row::ProfileRow,
        util::{AppComponent, impl_component},
    },
    anyhow::{Context, Result},
    foldhash::{HashMap, HashMapExt},
    glib::clone,
    gtk::prelude::{CheckButtonExt, EditableExt, WidgetExt},
    relm4::prelude::*,
    wordbase::{NormString, ProfileId},
    wordbase_engine::{EngineEvent, ProfileEvent},
};

mod ui;

#[derive(Debug)]
pub struct ManageProfiles {
    rows: HashMap<ProfileId, AsyncController<ProfileRow>>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    AddProfile,
    AddProfileName,
}

impl_component!(ManageProfiles);

impl AppComponent for ManageProfiles {
    type Args = ();
    type Msg = Msg;
    type Ui = ui::ManageProfiles;

    async fn init(
        (): Self::Args,
        ui: Self::Ui,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        ui.add_profile().connect_activated(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AddProfile)
        ));
        ui.add_profile_name().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AddProfileName)
        ));

        let mut model = Self {
            rows: HashMap::new(),
        };
        for &id in engine().profiles().keys() {
            model.add_row(&ui, &sender, id);
        }
        Self::update_add_profile_name(&ui);

        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Msg,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        match msg {
            Msg::AddProfile => {
                let name = NormString::new(ui.add_profile_name().text());
                let Some(name) = name else {
                    return Ok(());
                };

                let profile_id = current_profile_id();
                let Some(profile) = engine().profiles().get(&profile_id).cloned() else {
                    return Ok(());
                };

                let mut config = profile.config.clone();
                config.name = Some(name);
                engine()
                    .copy_profile(profile_id, config)
                    .await
                    .with_context(|| gettext("Failed to add profile"))?;
            }
            Msg::AddProfileName => {
                Self::update_add_profile_name(ui);
            }
        }
        Ok(())
    }

    async fn update_event(
        &mut self,
        event: AppEvent,
        sender: &AsyncComponentSender<Self>,
        ui: &Self::Root,
    ) -> Result<()> {
        match event {
            AppEvent::Engine(EngineEvent::Profile(ProfileEvent::Added(id))) => {
                self.add_row(ui, sender, id);
            }
            AppEvent::Engine(EngineEvent::Profile(ProfileEvent::Removed(id))) => {
                if let Some(row) = self.rows.remove(&id) {
                    ui.list().remove(row.widget());
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl ManageProfiles {
    fn update_add_profile_name(ui: &ui::ManageProfiles) {
        let name = NormString::new(ui.add_profile_name().text());
        if name.is_none() {
            ui.add_profile_name().set_css_classes(&["error"]);
        } else {
            ui.add_profile_name().set_css_classes(&[]);
        }
    }

    fn add_row(
        &mut self,
        ui: &ui::ManageProfiles,
        sender: &AsyncComponentSender<Self>,
        id: ProfileId,
    ) {
        let row = ProfileRow::builder()
            .launch(id)
            .forward(sender.output_sender(), |resp| resp);
        row.widget().current().set_group(Some(&ui.dummy_group()));
        ui.list().append(row.widget());
        self.rows.insert(id, row);
    }
}
