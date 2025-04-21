use {
    crate::{
        AppEvent, current_profile_id, engine, forward_events, gettext,
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
    type Init = ();
    type Input = Msg;
    type Root = ui::ManageProfiles;

    async fn init(
        (): (),
        root: ui::ManageProfiles,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        root.add_profile().connect_activated(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AddProfile)
        ));
        root.add_profile_name().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AddProfileName)
        ));

        let mut model = Self {
            rows: HashMap::new(),
        };
        for &id in engine().profiles().keys() {
            model.add_row(&root, &sender, id);
        }
        Self::update_add_profile_name(&root);

        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Msg,
        _sender: &AsyncComponentSender<Self>,
        root: &ui::ManageProfiles,
    ) -> Result<()> {
        match msg {
            Msg::AddProfile => {
                let name = NormString::new(root.add_profile_name().text());
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
                Self::update_add_profile_name(root);
            }
        }
        Ok(())
    }

    async fn update_event(
        &mut self,
        event: AppEvent,
        sender: &AsyncComponentSender<Self>,
        root: &ui::ManageProfiles,
    ) -> Result<()> {
        match event {
            AppEvent::Engine(EngineEvent::Profile(ProfileEvent::Added(id))) => {
                self.add_row(root, &sender, id);
            }
            AppEvent::Engine(EngineEvent::Profile(ProfileEvent::Removed(id))) => {
                if let Some(row) = self.rows.remove(&id) {
                    root.list().remove(row.widget());
                }
            }
            _ => {}
        }
        Ok(())
    }
}

impl ManageProfiles {
    fn update_add_profile_name(root: &ui::ManageProfiles) {
        let name = NormString::new(root.add_profile_name().text());
        if name.is_none() {
            root.add_profile_name().set_css_classes(&["error"]);
        } else {
            root.add_profile_name().set_css_classes(&[]);
        }
    }

    fn add_row(
        &mut self,
        root: &ui::ManageProfiles,
        sender: &AsyncComponentSender<Self>,
        id: ProfileId,
    ) {
        let row = ProfileRow::builder()
            .launch(id)
            .forward(sender.output_sender(), |resp| resp);
        row.widget().current().set_group(Some(&root.dummy_group()));
        root.list().append(row.widget());
        self.rows.insert(id, row);
    }
}
