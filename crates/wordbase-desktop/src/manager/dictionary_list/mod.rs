mod ui;

use std::sync::Arc;

use anyhow::{Context, Result};
use foldhash::{HashMap, HashMapExt};
use relm4::{
    adw::{gio, glib::clone, prelude::*},
    prelude::*,
};
use wordbase::{Dictionary, DictionaryId};
use wordbase_engine::{Engine, dictionary::Dictionaries};

use crate::manager::dictionary_row;

#[derive(Debug)]
pub struct Model {
    window: adw::Window,
    dictionaries: HashMap<DictionaryId, Controller<dictionary_row::Model>>,
    sorting_id: Option<DictionaryId>,
}

#[derive(Debug, Clone)]
pub enum Msg {
    Add(Dictionary),
    SetEnabled(DictionaryId, bool),
    SetSorting(Option<DictionaryId>),
    Remove(DictionaryId),
    #[doc(hidden)]
    AskImport,
}

#[derive(Debug, Clone)]
pub enum Response {
    Import(gio::ListModel),
    SetEnabled(DictionaryId, bool),
    SetSorting(Option<DictionaryId>),
    Remove(DictionaryId),
}

impl Component for Model {
    type Init = (adw::Window, Arc<Dictionaries>);
    type Input = Msg;
    type Output = Response;
    type CommandOutput = ();
    type Root = ui::Dictionaries;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::Dictionaries::new()
    }

    fn init(
        (window, dictionaries): Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let mut model = Self {
            window,
            dictionaries: HashMap::new(),
            sorting_id: dictionaries.sorting_id,
        };

        root.import_button().connect_activated(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskImport)
        ));

        for dictionary in dictionaries.by_id.values() {
            let row = make_row(&model, dictionary.clone(), &sender);
            root.list()
                .insert(row.widget(), root.import_button().index());
            model.dictionaries.insert(dictionary.id, row);
        }

        ComponentParts { model, widgets: () }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            Msg::Add(dictionary) => {
                let dictionary_id = dictionary.id;
                let row = make_row(self, dictionary, &sender);
                self.dictionaries.insert(dictionary_id, row);
            }
            Msg::SetEnabled(dictionary_id, enabled) => {
                if let Some(row) = self.dictionaries.get(&dictionary_id) {
                    _ = row.sender().send(dictionary_row::Msg::SetEnabled(enabled));
                }
            }
            Msg::SetSorting(dictionary_id) => {
                for (test_id, dictionary) in &self.dictionaries {
                    _ = dictionary.sender().send(dictionary_row::Msg::SetSorting(
                        dictionary_id == Some(*test_id),
                    ));
                }
            }
            Msg::Remove(dictionary_id) => {
                if let Some(row) = self.dictionaries.remove(&dictionary_id) {
                    root.list().remove(row.widget());
                }
            }
            Msg::AskImport => {
                root.import_dialog().open_multiple(
                    Some(&self.window),
                    None::<&gio::Cancellable>,
                    clone!(
                        #[strong]
                        sender,
                        move |result| if let Ok(files) = result {
                            _ = sender.output(Response::Import(files));
                        }
                    ),
                );
            }
        }

        self.update_view(widgets, sender);
    }
}

fn make_row(
    model: &Model,
    dictionary: Dictionary,
    sender: &ComponentSender<Model>,
) -> Controller<dictionary_row::Model> {
    let dictionary_id = dictionary.id;
    let sorting = model.sorting_id == Some(dictionary_id);
    dictionary_row::Model::builder()
        .launch((model.window.clone(), dictionary, sorting))
        .forward(sender.output_sender(), move |resp| match resp {
            dictionary_row::Response::SetEnabled(enabled) => {
                Response::SetEnabled(dictionary_id, enabled)
            }
            dictionary_row::Response::SetSorting(false) => Response::SetSorting(None),
            dictionary_row::Response::SetSorting(true) => Response::SetSorting(Some(dictionary_id)),
            dictionary_row::Response::Remove => Response::Remove(dictionary_id),
        })
}

pub async fn apply(msg: Response, engine: &Engine) -> Result<()> {
    match msg {
        Response::Import(_) => todo!(),
        Response::SetEnabled(dictionary_id, false) => engine
            .disable_dictionary(dictionary_id)
            .await
            .context("failed to disable dictionary"),
        Response::SetEnabled(dictionary_id, true) => engine
            .enable_dictionary(dictionary_id)
            .await
            .context("failed to enable dictionary"),
        Response::SetSorting(dictionary_id) => engine
            .set_sorting_dictionary(dictionary_id)
            .await
            .context("failed to set sorting dictionary"),
        Response::Remove(dictionary_id) => engine
            .remove_dictionary(dictionary_id)
            .await
            .context("failed to remove dictionary"),
    }
}
