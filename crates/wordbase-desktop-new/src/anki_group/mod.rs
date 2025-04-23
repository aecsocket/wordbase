use adw::prelude::*;
use anyhow::Result;
use glib::clone;
use relm4::prelude::*;
use tokio_util::task::AbortOnDropHandle;

use crate::{
    AppEvent, current_profile, engine,
    util::{AppComponent, impl_component},
};

mod ui;

#[derive(Debug)]
pub struct AnkiGroup {
    connect_task: Option<AbortOnDropHandle<()>>,
    note_fields: Vec<adw::ComboRow>,
}

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {
    Connect,
    UpdateRoot,
}

impl_component!(AnkiGroup);

impl AppComponent for AnkiGroup {
    type Args = ();
    type Msg = Msg;
    type Ui = ui::AnkiGroup;

    async fn init(
        (): Self::Args,
        ui: Self::Ui,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let config = engine().anki_config();
        ui.server_url().set_text(&config.server_url);
        ui.server_url().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::Connect)
        ));
        ui.api_key().set_text(&config.api_key);
        ui.api_key().connect_changed(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::UpdateRoot)
        ));

        let mut model = Self {
            connect_task: None,
            note_fields: Vec::new(),
        };
        model.update_root(&ui);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Self::Msg,
        sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        match msg {
            Msg::Connect => {
                let server_url = ui.server_url().text();
                let api_key = ui.api_key().text();
                self.connect_task = Some(AbortOnDropHandle::new(tokio::spawn(clone!(
                    #[strong]
                    sender,
                    async move {
                        _ = engine()
                            .anki_connect(server_url.as_str(), api_key.as_str())
                            .await;
                        sender.input(Msg::UpdateRoot);
                    }
                ))));
            }
            Msg::UpdateRoot => {
                self.update_root(ui);
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
        if matches!(event, AppEvent::ProfileIdSet) {
            self.update_root(ui);
        }
        Ok(())
    }
}

impl AnkiGroup {
    fn update_root(&mut self, ui: &ui::AnkiGroup) {
        let profile = current_profile();

        clear_string_list(&ui.deck_model());
        clear_string_list(&ui.note_type_model());

        let engine = engine();
        match engine.anki_state() {
            Ok(anki) => {
                ui.connected().set_visible(true);
                ui.disconnected().set_visible(false);

                for (index, deck_name) in anki.decks.iter().enumerate() {
                    ui.deck_model().append(deck_name);
                    if profile.config.anki_deck.as_ref().map(|s| s.as_str())
                        == Some(deck_name.as_str())
                    {
                        ui.deck().set_selected(
                            u32::try_from(index).expect("should not exceed `u32::MAX` decks"),
                        );
                    }
                }

                let mut model = None;
                for (index, (note_type_name, this_model)) in anki.models.iter().enumerate() {
                    ui.note_type_model().append(note_type_name);
                    if profile.config.anki_note_type.as_ref().map(|s| s.as_str())
                        == Some(note_type_name.as_str())
                    {
                        ui.note_type().set_selected(
                            u32::try_from(index).expect("should not exceed `u32::MAX` note types"),
                        );
                        model = Some(this_model);
                    }
                }

                ui.note_fields().set_visible(false);
                for note_field in self.note_fields.drain(..) {
                    ui.note_fields().remove(&note_field);
                }

                if let Some(model) = model {
                    ui.note_fields().set_visible(true);
                    for field_name in &model.field_names {
                        let row = adw::ComboRow::builder()
                            .title(field_name.as_str())
                            .model(&ui.field_content_model())
                            .build();
                        ui.note_fields().add_row(&row);
                        self.note_fields.push(row);
                    }
                }
            }
            Err(err) => {
                ui.connected().set_visible(false);
                ui.disconnected().set_visible(true);
                ui.disconnected()
                    .set_tooltip_text(Some(&format!("{err:?}")));
                ui.note_fields().set_visible(false);
            }
        };
    }
}

fn clear_string_list(list: &gtk::StringList) {
    for i in (0..list.n_items()).rev() {
        list.remove(i);
    }
}
