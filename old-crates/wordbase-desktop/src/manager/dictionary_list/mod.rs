mod ui;

use {
    crate::{AppEvent, forward_events, manager::dictionary_row},
    foldhash::{HashMap, HashMapExt},
    relm4::{
        adw::{glib::clone, prelude::*},
        prelude::*,
    },
    std::sync::Arc,
    wordbase::{Dictionary, DictionaryId},
    wordbase_engine::Engine,
};

#[derive(Debug)]
pub struct Model {
    dictionaries: HashMap<DictionaryId, AsyncController<dictionary_row::Model>>,
    engine: Engine,
    window: gtk::Window,
    toaster: adw::ToastOverlay,
}

#[derive(Debug, Clone)]
#[doc(hidden)]
pub enum Msg {
    AskImport,
}

impl AsyncComponent for Model {
    type Init = (Engine, gtk::Window, adw::ToastOverlay);
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::DictionaryList;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::DictionaryList::new()
    }

    async fn init(
        (engine, window, toaster): Self::Init,
        root: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        forward_events(&sender);

        root.import_button().connect_activated(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskImport)
        ));

        let mut model = Self {
            dictionaries: HashMap::new(),
            engine,
            window,
            toaster,
        };
        for dictionary in model.engine.dictionaries().values() {
            add_row(&mut model, &root, dictionary.clone());
        }
        AsyncComponentParts { model, widgets: () }
    }

    async fn update_with_view(
        &mut self,
        _widgets: &mut Self::Widgets,
        message: Self::Input,
        _sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            Msg::AskImport => {
                import(self, root).await;
            }
        }
    }

    async fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match message {
            AppEvent::DictionaryRemoved(id) => {
                if let Some(row) = self.dictionaries.remove(&id) {
                    root.list().remove(row.widget());
                }
            }
            _ => {}
        }
    }
}

fn add_row(model: &mut Model, root: &ui::DictionaryList, dictionary: Arc<Dictionary>) {
    let dictionary_id = dictionary.id;
    let row = dictionary_row::Model::builder()
        .launch((
            model.engine.clone(),
            model.window.clone(),
            model.toaster.clone(),
            dictionary,
        ))
        .detach();
    root.list()
        .insert(row.widget(), root.import_button().index());
    model.dictionaries.insert(dictionary_id, row);
}

async fn import(model: &Model, root: &ui::DictionaryList) {
    let Ok(files) = root
        .import_dialog()
        .open_multiple_future(Some(&model.window))
        .await
    else {
        return;
    };

    todo!();
}
