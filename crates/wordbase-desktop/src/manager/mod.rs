mod dictionary_row;
mod error_dialog;
mod ui;

use crate::gettext;
use anyhow::{Context, Result};
use dictionary_row::DictionaryRow;
use ui::Manager;
use wordbase::{DictionaryId, DictionaryState, ProfileId, ProfileMeta};
use wordbase_engine::{Engine, Event};

use adw::{gio, gtk, prelude::*};

type BiHashMap<L, R> =
    bimap::BiHashMap<L, R, foldhash::fast::RandomState, foldhash::fast::RandomState>;

pub async fn ui(engine: Engine, window: gtk::Window) -> Result<gtk::Widget> {
    let ui = Manager::new();
    let mut profiles = BiHashMap::<ProfileId, u32>::default();

    for profile in engine
        .profiles()
        .await
        .context("failed to fetch initial profiles")?
    {
        profiles.insert(profile.id, ui.profiles().n_items());
        ui.profiles().append(profile_name(&profile.meta));
    }

    {
        let mut engine = engine.clone();
        let ui = ui.clone();
        glib::spawn_future_local(async move {
            loop {
                match engine.recv_event.recv().await {
                    Ok(Event::SyncDictionaries(dictionaries)) => {
                        present_dictionaries(&engine, &ui, dictionaries);
                    }
                    Ok(Event::ProfileAdded { profile }) => {
                        profiles.insert(profile.id, ui.profiles().n_items());
                        ui.profiles().append(profile_name(&profile.meta));
                    }
                    Ok(Event::ProfileRemoved { profile_id }) => {
                        if let Some((_, position)) = profiles.remove_by_left(&profile_id) {
                            ui.profiles().remove(position);
                        }
                    }
                    Err(_) => {}
                }
            }
        });
    }

    {
        let ui = ui.clone();
        ui.import_dictionary().connect_activated(move |_| {
            glib::spawn_future_local(import_dictionaries(
                engine.clone(),
                ui.clone(),
                window.clone(),
            ));
        });
    }

    Ok(ui.upcast())
}

fn profile_name(meta: &ProfileMeta) -> &str {
    meta.name
        .as_deref()
        .unwrap_or_else(|| gettext("Default Profile"))
}

fn present_dictionaries(engine: &Engine, ui: &Manager, dictionaries: Vec<DictionaryState>) {
    ui.dictionaries().remove_all();
    for dictionary in dictionaries {
        let row = DictionaryRow::new();
        ui.dictionaries()
            .insert(&row, ui.import_dictionary().index());
        row.set_title(&dictionary.meta.name);
        row.set_subtitle(&dictionary.meta.version);
        row.enabled_bin().set_visible(true);

        let engine = engine.clone();
        row.delete().connect_clicked(move |_| {
            let engine = engine.clone();
            let row = row.clone();
            glib::spawn_future_local(async move {
                delete_dictionary(&engine, dictionary.id, &row).await;
            });
        });
    }
}

async fn delete_dictionary(engine: &Engine, dictionary_id: DictionaryId, row: &DictionaryRow) {
    const CANCEL: &str = "cancel";
    const DELETE: &str = "delete";

    let dialog = adw::AlertDialog::new(
        Some(gettext("Delete Dictionary?")),
        Some(gettext("Are you sure you want to delete this dictionary?")),
    );
    dialog.add_response(CANCEL, gettext("Cancel"));
    dialog.add_response(DELETE, gettext("Delete"));
    dialog.set_response_appearance("delete", adw::ResponseAppearance::Destructive);

    let response = dialog.choose_future(row).await;
    if response.as_str() == DELETE {
        _ = engine.delete_dictionary(dictionary_id).await;
    }
}

async fn import_dictionaries(engine: Engine, ui: Manager, window: gtk::Window) {
    let Ok(files) = gtk::FileDialog::builder()
        .title(gettext("Pick Dictionaries"))
        .accept_label(gettext("Import"))
        .build()
        .open_multiple_future(Some(&window))
        .await
    else {
        return;
    };

    for file in &files {
        let file = file.expect("should not be mutated during iteration");
        let file = file
            .downcast::<gio::File>()
            .expect("object should be a file");

        let engine = engine.clone();
        tokio::spawn(async move {
            let _import_permit = engine
                .import_concurrency
                .acquire()
                .await
                .context("failed to acquire import permit")?;
            // let (data, _) = file
            //     .load_bytes_future()
            //     .await
            //     .context("failed to read file into memory")?;

            // let (send_tracker, recv_tracker) = oneshot::channel();
            // engine.import_dictionary(&data, send_tracker).await;

            anyhow::Ok(())
        });
    }
}
