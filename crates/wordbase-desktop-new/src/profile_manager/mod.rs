use {
    crate::{
        AppEvent, current_profile, current_profile_id, engine, gettext,
        profile_row::{self, ProfileRow},
        util::{AppComponent, impl_component},
    },
    adw::prelude::*,
    anyhow::{Context, Result},
    foldhash::{HashMap, HashMapExt},
    glib::clone,
    relm4::prelude::*,
    std::sync::Arc,
    wordbase::{NormString, Profile, ProfileId},
    wordbase_engine::EngineEvent,
};

mod ui;

#[derive(Debug)]
pub struct ProfileManager {
    rows: HashMap<ProfileId, AsyncController<ProfileRow>>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    Create,
}

impl_component!(ProfileManager);

impl AppComponent for ProfileManager {
    type Args = ();
    type Msg = Msg;
    type Ui = ui::ProfileManager;

    async fn init(
        (): Self::Args,
        ui: Self::Ui,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        ui.create().connect_clicked(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::Create),
        ));

        let mut model = Self {
            rows: HashMap::new(),
        };
        for profile in engine().profiles().values() {
            model.add_row(&ui, &sender, profile.clone());
        }

        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Msg,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        match msg {
            Msg::Create => {
                let current_profile = current_profile();
                let new_name = format!("{}*", profile_row::name_of(&current_profile));
                let new_name = NormString::new(new_name)
                    .expect("new name should be a valid normalized string");
                engine()
                    .copy_profile(current_profile_id(), Some(new_name))
                    .await
                    .with_context(|| gettext("Failed to add profile"))?;
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
            AppEvent::Engine(
                EngineEvent::ProfileAdded { id }
                | EngineEvent::ProfileCopied {
                    src_id: _,
                    new_id: id,
                },
            ) => {
                if let Some(profile) = engine().profiles().get(&id) {
                    self.add_row(ui, sender, profile.clone());
                }
            }
            AppEvent::Engine(EngineEvent::ProfileRemoved { id }) => {
                if let Some(row) = self.rows.remove(&id) {
                    ui.list().remove(row.widget());
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl ProfileManager {
    fn add_row(
        &mut self,
        ui: &ui::ProfileManager,
        sender: &AsyncComponentSender<Self>,
        profile: Arc<Profile>,
    ) {
        let profile_id = profile.id;
        let row = ProfileRow::builder()
            .launch(profile)
            .forward(sender.output_sender(), |resp| resp);
        row.widget().current().set_group(Some(&ui.dummy_group()));
        ui.list().append(row.widget());
        self.rows.insert(profile_id, row);
    }
}
