use std::sync::Arc;

use adw::prelude::*;
use glib::clone;
use relm4::prelude::*;
use tokio_util::task::AbortOnDropHandle;
use wordbase_engine::anki::AnkiConfig;

use crate::{AppEvent, PROFILE, current_profile_id, engine, forward_events, settings};

mod ui;

#[derive(Debug)]
pub struct AnkiGroup {
    connect_task: Option<AbortOnDropHandle<()>>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    Connect,
    UpdateRoot,
}

impl AsyncComponent for AnkiGroup {
    type Init = ();
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::AnkiGroup;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::AnkiGroup::new()
    }

    async fn init(
        (): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);

        let config = engine().anki_config();
        root.server_url().set_text(&config.server_url);
        root.server_url().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::Connect)
        ));
        root.api_key().set_text(&config.api_key);
        root.api_key().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::UpdateRoot)
        ));
        Self::update_root(&root);

        settings().connect_changed(
            Some(PROFILE),
            clone!(
                #[strong]
                sender,
                move |_, _| sender.input(Msg::UpdateRoot)
            ),
        );

        AsyncComponentParts {
            model: Self { connect_task: None },
            widgets: (),
        }
    }

    async fn update_with_view(
        &mut self,
        (): &mut Self::Widgets,
        msg: Self::Input,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match msg {
            Msg::Connect => {
                let server_url = root.server_url().text();
                let api_key = root.api_key().text();
                self.connect_task = Some(AbortOnDropHandle::new(tokio::spawn(async move {
                    _ = engine()
                        .connect_anki(Arc::new(AnkiConfig {
                            server_url: Arc::from(server_url.to_string()),
                            api_key: Arc::from(api_key.to_string()),
                        }))
                        .await;
                    sender.input(Msg::UpdateRoot);
                })));
            }
            Msg::UpdateRoot => {
                Self::update_root(root);
            }
        }
    }
}

impl AnkiGroup {
    fn update_root(root: &ui::AnkiGroup) {
        // TODO
        clear_string_list(&root.field_content_model());

        clear_string_list(&root.deck_model());
        clear_string_list(&root.note_type_model());

        let engine = engine();
        match engine.anki_state() {
            Ok(anki) => {
                root.connected().set_visible(true);
                root.disconnected().set_visible(false);

                let profiles = engine.profiles();
                let Some(profile) = profiles.get(&current_profile_id()) else {
                    return;
                };

                for (index, deck_name) in anki.decks.iter().enumerate() {
                    root.deck_model().append(deck_name);
                    if profile.config.anki_deck.as_ref().map(|s| s.as_str())
                        == Some(deck_name.as_str())
                    {
                        root.deck().set_selected(index as u32);
                    }
                }

                let mut model = None;
                for (index, (model_name, this_model)) in anki.models.iter().enumerate() {
                    root.note_type_model().append(model_name);
                    if profile.config.anki_model.as_ref().map(|s| s.as_str())
                        == Some(model_name.as_str())
                    {
                        root.note_type().set_selected(index as u32);
                        model = Some(this_model);
                    }
                }

                root.note_fields().set_visible(false);
                if let Some(model) = model {
                    root.note_fields().set_visible(true);
                    for field_name in &model.field_names {
                        let row = adw::ComboRow::builder()
                            .title(field_name.as_str())
                            .model(&root.field_content_model())
                            .build();
                        root.note_fields().add_row(&row);
                    }
                }
            }
            Err(err) => {
                root.connected().set_visible(false);
                root.disconnected().set_visible(true);
                root.disconnected()
                    .set_tooltip_text(Some(&format!("{err:?}")));
                root.note_fields().set_visible(false);
            }
        };
    }
}

fn clear_string_list(list: &gtk::StringList) {
    for i in (0..list.n_items()).rev() {
        list.remove(i);
    }
}
