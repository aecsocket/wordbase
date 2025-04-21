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
    tracing::{debug, info, warn},
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
            AppEvent::Engine(EngineEvent::Dictionary(DictionaryEvent::Added(dictionary))) => {
                self.add_row(ui, sender, dictionary);
            }
            AppEvent::Engine(EngineEvent::Dictionary(DictionaryEvent::Removed(id))) => {
                if let Some(row) = self.rows.remove(&id) {
                    ui.list().remove(row.widget());
                }
            }
            AppEvent::Engine(EngineEvent::Dictionary(DictionaryEvent::PositionSet(_, _))) => {
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

    let dst_row = ui
        .list()
        .row_at_y(y as i32)
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

    ui.list().remove(&src_row);
    ui.list().insert(&src_row, dst_index);
    src_row.set_state_flags(gtk::StateFlags::NORMAL, true);

    ui.list().remove(&dst_row);
    ui.list().insert(&dst_row, src_index);

    glib::spawn_future(async move {
        if let Err(err) = swap_positions(&src_dict, &dst_dict)
            .await
            .with_context(|| gettext("Failed to set dictionary position"))
        {
            _ = sender.output(err);
        }
    });
    Ok(())
}

async fn swap_positions(src: &Dictionary, dst: &Dictionary) -> Result<()> {
    let (src_pos, dst_pos) = if src.position == dst.position {
        warn!(
            "{:?} and {:?} have the same position {}, bumping the former up one",
            src.meta.name, dst.meta.name, src.position
        );
        (src.position + 1, src.position)
    } else {
        (src.position, dst.position)
    };

    debug!(
        "Moving {:?} to {}, and {:?} to {}",
        src.meta.name, dst_pos, dst.meta.name, src_pos
    );

    engine()
        .set_dictionary_position(src.id, dst_pos)
        .await
        .context("failed to set source dictionary position")?;
    engine()
        .set_dictionary_position(dst.id, src_pos)
        .await
        .context("failed to set destination dictionary position")?;

    Ok(())
}
