use anyhow::Context;
use foldhash::{HashMap, HashMapExt};
use glib::clone;
use gtk::prelude::{CheckButtonExt, EditableExt, WidgetExt};
use relm4::prelude::*;
use wordbase::{NormString, ProfileId};
use wordbase_engine::{EngineEvent, ProfileEvent};

use crate::{
    AppEvent, CURRENT_PROFILE_ID, engine, forward_events, gettext, handle_result,
    profile_row::ProfileRow,
};

mod ui;

#[derive(Debug)]
pub struct ManageProfiles {
    window: gtk::Window,
    rows: HashMap<ProfileId, AsyncController<ProfileRow>>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    AddProfile,
    AddProfileName,
}

impl AsyncComponent for ManageProfiles {
    type Init = gtk::Window;
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::ManageProfiles;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::ManageProfiles::new()
    }

    async fn init(
        window: Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);

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
            window,
            rows: HashMap::new(),
        };
        for profile_id in engine().profiles().keys() {
            model.add_row(&root, *profile_id);
        }
        Self::update_add_profile_name(&root);

        AsyncComponentParts { model, widgets: () }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        msg: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            Msg::AddProfile => {
                let name = NormString::new(root.add_profile_name().text());
                let Some(name) = name else {
                    return;
                };

                let profile_id = *CURRENT_PROFILE_ID.read();
                let Some(profile) = engine().profiles().get(&profile_id).cloned() else {
                    return;
                };

                let mut config = profile.config.clone();
                config.name = Some(name);
                handle_result(
                    engine()
                        .copy_profile(profile_id, config)
                        .await
                        .with_context(|| gettext("Failed to add profile")),
                );
            }
            Msg::AddProfileName => Self::update_add_profile_name(root),
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        event: Self::CommandOutput,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match event {
            AppEvent::Engine(EngineEvent::Profile(ProfileEvent::Added(profile_id))) => {
                self.add_row(root, profile_id)
            }
            AppEvent::Engine(EngineEvent::Profile(ProfileEvent::Removed(profile_id))) => {
                if let Some(row) = self.rows.remove(&profile_id) {
                    root.list().remove(row.widget());
                }
            }
            _ => {}
        }
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

    fn add_row(&mut self, root: &ui::ManageProfiles, profile_id: ProfileId) {
        let row = ProfileRow::builder()
            .launch((self.window.clone(), profile_id))
            .detach();
        row.widget().current().set_group(Some(&root.dummy_group()));
        root.list().append(row.widget());
        self.rows.insert(profile_id, row);
    }
}
