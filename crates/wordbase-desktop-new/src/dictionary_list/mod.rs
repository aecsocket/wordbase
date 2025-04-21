mod ui;

use {
    crate::{AppEvent, app_window, dictionary_row::DictionaryRow, engine, forward_events},
    foldhash::{HashMap, HashMapExt},
    relm4::{
        adw::{glib::clone, prelude::*},
        prelude::*,
    },
    wordbase::DictionaryId,
    wordbase_engine::{DictionaryEvent, EngineEvent},
};

#[derive(Debug)]
pub struct DictionaryList {
    rows: HashMap<DictionaryId, AsyncController<DictionaryRow>>,
}

#[derive(Debug, Clone)]
#[doc(hidden)]
pub enum Msg {
    AskImport,
}

impl AsyncComponent for DictionaryList {
    type Init = ();
    type Input = Msg;
    type Output = ();
    type CommandOutput = AppEvent;
    type Root = ui::DictionaryList;
    type Widgets = ();

    fn init_root() -> Self::Root {
        ui::DictionaryList::new()
    }

    async fn init(
        (): Self::Init,
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
            rows: HashMap::new(),
        };
        for &id in engine().dictionaries().keys() {
            model.add_row(&root, &sender, id);
        }
        AsyncComponentParts { model, widgets: () }
    }

    async fn update_with_view(
        &mut self,
        (): &mut Self::Widgets,
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
        (): &mut Self::Widgets,
        event: Self::CommandOutput,
        sender: AsyncComponentSender<Self>,
        root: &Self::Root,
    ) {
        match event {
            AppEvent::Engine(EngineEvent::Dictionary(DictionaryEvent::Added(id))) => {
                self.add_row(root, &sender, id);
            }
            AppEvent::Engine(EngineEvent::Dictionary(DictionaryEvent::Removed(id))) => {
                if let Some(row) = self.rows.remove(&id) {
                    root.list().remove(row.widget());
                }
            }
            _ => {}
        }
    }
}

impl DictionaryList {
    fn add_row(
        &mut self,
        root: &ui::DictionaryList,
        sender: &AsyncComponentSender<Self>,
        id: DictionaryId,
    ) {
        let row = DictionaryRow::builder()
            .launch(id)
            .forward(sender.output_sender(), |resp| resp);
        root.list()
            .insert(row.widget(), root.import_button().index());
        self.rows.insert(id, row);
    }
}

async fn import(model: &DictionaryList, root: &ui::DictionaryList) {
    let Ok(files) = root
        .import_dialog()
        .open_multiple_future(Some(&app_window()))
        .await
    else {
        return;
    };

    todo!();
}
