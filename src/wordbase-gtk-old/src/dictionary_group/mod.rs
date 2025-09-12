mod ui;

use {
    crate::{
        AppEvent, app_window,
        dictionary_row::{self, DictionaryRow},
        engine, gettext,
        util::{AppComponent, impl_component},
    },
    anyhow::{Context, Result},
    arc_swap::ArcSwap,
    foldhash::{HashMap, HashMapExt},
    relm4::{
        adw::{gdk, glib::clone, prelude::*},
        prelude::*,
    },
    std::sync::Arc,
    tracing::{debug, warn},
    wordbase::{Dictionary, DictionaryId},
    wordbase_engine::{DictionaryEvent, EngineEvent},
};

#[derive(Debug)]
pub struct DictionaryGroup {
    rows: HashMap<DictionaryId, AsyncController<DictionaryRow>>,
}

#[derive(Debug, Clone)]
#[doc(hidden)]
pub enum Msg {
    AskImport,
}

impl_component!(DictionaryGroup);

impl AppComponent for DictionaryGroup {
    type Args = ();
    type Msg = Msg;
    type Ui = ui::DictionaryGroup;

    async fn init(
        (): Self::Args,
        ui: Self::Root,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        let drop_target = gtk::DropTarget::new(
            dictionary_row::ui::DictionaryRow::static_type(),
            gdk::DragAction::MOVE,
        );
        drop_target.connect_drop(clone!(
            #[strong]
            ui,
            #[strong]
            sender,
            move |_, src_row, _x, y| {
                match drag_drop(&ui, src_row, y, sender.clone()) {
                    Ok(()) => true,
                    Err(err) => {
                        warn!("Failed to drag and drop dictionary: {err:?}");
                        false
                    }
                }
            }
        ));
        ui.list().add_controller(drop_target);

        ui.import_button().connect_activated(clone!(
            #[strong]
            sender,
            move |_| sender.input(Msg::AskImport)
        ));

        let mut model = Self {
            rows: HashMap::new(),
        };
        model.sync(&ui, &sender);
        AsyncComponentParts { model, widgets: () }
    }

    async fn update(
        &mut self,
        msg: Self::Msg,
        _sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        match msg {
            Msg::AskImport => {
                self.import(ui).await;
            }
        }
        Ok(())
    }

    async fn update_event(
        &mut self,
        event: AppEvent,
        sender: &AsyncComponentSender<Self>,
        ui: &Self::Ui,
    ) -> Result<()> {
        match event {
            AppEvent::Engine(EngineEvent::Dictionary(DictionaryEvent::Added { id })) => {
                if let Some(dictionary) = engine().dictionaries().get(&id) {
                    self.add_row(ui, sender, dictionary.clone());
                }
            }
            AppEvent::Engine(EngineEvent::Dictionary(DictionaryEvent::Removed { id })) => {
                if let Some(row) = self.rows.remove(&id) {
                    ui.list().remove(row.widget());
                }
            }
            AppEvent::Engine(EngineEvent::Dictionary(DictionaryEvent::PositionsSwapped {
                ..
            })) => {
                self.sync(ui, sender);
            }
            _ => {}
        }
        Ok(())
    }
}

impl DictionaryGroup {
    fn sync(&mut self, ui: &ui::DictionaryGroup, sender: &AsyncComponentSender<Self>) {
        for (_, row) in self.rows.drain() {
            ui.list().remove(row.widget());
        }
        for dictionary in engine().dictionaries().values() {
            self.add_row(ui, sender, dictionary.clone());
        }
    }

    fn add_row(
        &mut self,
        ui: &ui::DictionaryGroup,
        sender: &AsyncComponentSender<Self>,
        dictionary: Arc<Dictionary>,
    ) {
        let dictionary_id = dictionary.id;
        let row = DictionaryRow::builder()
            .launch((Arc::new(ArcSwap::new(dictionary)), ui.list()))
            .forward(sender.output_sender(), |resp| resp);
        ui.list().insert(row.widget(), ui.import_button().index());
        self.rows.insert(dictionary_id, row);
    }

    async fn import(&self, ui: &ui::DictionaryGroup) {
        let Ok(files) = ui
            .import_dialog()
            .open_multiple_future(Some(&app_window()))
            .await
        else {
            return;
        };

        for file in &files {
            let file = file
                .expect("list should not be mutated while iterating")
                .downcast::<gio::File>()
                .expect("list item should be a file");
            let archive = file.load_bytes_future().await;
        }

        todo!();
    }
}

fn drag_drop(
    ui: &ui::DictionaryGroup,
    src_row: &glib::Value,
    y: f64,
    sender: AsyncComponentSender<DictionaryGroup>,
) -> Result<()> {
    let src_row = src_row
        .get::<dictionary_row::ui::DictionaryRow>()
        .expect("should have been set to a widget in drag prepare");
    let src_dict = src_row
        .dictionary()
        .context("source row has no dictionary")?;
    let src_index = src_row.index();

    #[expect(clippy::cast_possible_truncation, reason = "truncation is acceptable")]
    let y = y as i32;
    let dst_row = ui
        .list()
        .row_at_y(y)
        .context("no destination row at Y position")?
        .downcast::<dictionary_row::ui::DictionaryRow>()
        .ok()
        .context("destination row is not a dictionary row")?;
    let dst_dict = dst_row
        .dictionary()
        .context("destination row has no dictionary")?;
    let dst_index = dst_row.index();

    if src_index == dst_index {
        return Ok(());
    }

    debug!(
        "Swapping positions of {:?} and {:?}",
        src_dict.meta.name, dst_dict.meta.name
    );

    ui.list().remove(&src_row);
    ui.list().insert(&src_row, dst_index);
    src_row.set_state_flags(gtk::StateFlags::NORMAL, true);

    ui.list().remove(&dst_row);
    ui.list().insert(&dst_row, src_index);

    glib::spawn_future(async move {
        if let Err(err) = engine()
            .swap_dictionary_positions(src_dict.id, dst_dict.id)
            .await
            .with_context(|| gettext("Failed to swap dictionary positions"))
        {
            _ = sender.output(err);
        }
    });
    Ok(())
}
