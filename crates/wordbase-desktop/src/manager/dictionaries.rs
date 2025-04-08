use std::sync::Arc;

use anyhow::{Context, Result};
use foldhash::{HashMap, HashMapExt};
use relm4::{adw::prelude::*, prelude::*};
use wordbase::DictionaryId;
use wordbase_engine::{Engine, dictionary::Dictionaries};

use crate::manager::dictionary_row;

#[derive(Debug)]
pub struct Model {
    window: adw::Window,
    dictionaries: HashMap<DictionaryId, Controller<dictionary_row::Model>>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    SetEnabled(DictionaryId, bool),
    SetSorting(Option<DictionaryId>),
    Remove(DictionaryId),
}

#[relm4::component(pub)]
impl Component for Model {
    type Init = (adw::Window, Arc<Dictionaries>);
    type Input = Msg;
    type Output = Msg;
    type CommandOutput = ();

    view! {
        gtk::ListBox {
            set_css_classes: &["boxed-list"],
        }
    }

    fn init(
        (window, dictionaries): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            window: window.clone(),
            dictionaries: HashMap::new(),
        };
        let widgets = view_output!();

        for dictionary in dictionaries.by_id.values() {
            let dictionary_id = dictionary.id;
            let row = dictionary_row::Model::builder()
                .launch(dictionary_row::Config {
                    window: window.clone(),
                    dictionary: dictionary.clone(),
                    is_sorting: dictionaries.sorting_id == Some(dictionary_id),
                })
                .forward(sender.output_sender(), move |resp| match resp {
                    dictionary_row::Response::SetEnabled(enabled) => {
                        Msg::SetEnabled(dictionary_id, enabled)
                    }
                    dictionary_row::Response::SetSorting(false) => Msg::SetSorting(None),
                    dictionary_row::Response::SetSorting(true) => {
                        Msg::SetSorting(Some(dictionary_id))
                    }
                    dictionary_row::Response::Remove => Msg::Remove(dictionary_id),
                });
            root.append(row.widget());
            model.dictionaries.insert(dictionary.id, row);
        }

        ComponentParts { model, widgets }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            Msg::SetEnabled(id, enabled) => {
                if let Some(row) = self.dictionaries.get(&id) {
                    _ = row.sender().send(dictionary_row::Msg::SetEnabled(enabled));
                }
            }
            Msg::SetSorting(sorting_id) => {
                for (dictionary_id, dictionary) in &self.dictionaries {
                    _ = dictionary.sender().send(dictionary_row::Msg::SetSorting(
                        sorting_id == Some(*dictionary_id),
                    ));
                }
            }
            Msg::Remove(id) => {
                if let Some(row) = self.dictionaries.remove(&id) {
                    root.remove(row.widget());
                }
            }
        }

        self.update_view(widgets, sender);
    }
}

pub async fn apply(msg: Msg, engine: &Engine) -> Result<()> {
    match msg {
        Msg::SetEnabled(id, false) => engine
            .disable_dictionary(id)
            .await
            .context("failed to disable dictionary"),
        Msg::SetEnabled(id, true) => engine
            .enable_dictionary(id)
            .await
            .context("failed to enable dictionary"),
        Msg::SetSorting(id) => engine
            .set_sorting_dictionary(id)
            .await
            .context("failed to set sorting dictionary"),
        Msg::Remove(id) => engine
            .remove_dictionary(id)
            .await
            .context("failed to remove dictionary"),
    }
}
