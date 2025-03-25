mod dictionary_row;
mod error_dialog;
mod ui;

use crate::gettext;
use anyhow::{Context, Result};
use dictionary_row::DictionaryRow;
use error_dialog::ErrorDialog;
use ui::Manager;
use wordbase_engine::Engine;

use adw::{gio, gtk, prelude::*};

pub fn ui(engine: Engine, window: gtk::Window) -> gtk::Widget {
    let ui = Manager::new();

    let row = DictionaryRow::new();
    ui.dictionaries()
        .insert(&row, ui.import_dictionary().index());
    row.set_title("Jitendex");
    row.set_subtitle("2025.02.01");
    row.enabled().set_visible(true);

    let row = DictionaryRow::new();
    ui.dictionaries()
        .insert(&row, ui.import_dictionary().index());
    row.set_title("JMnedict");
    row.set_subtitle("version");
    row.importing().set_visible(true);
    row.progress().set_visible(true);
    row.progress().set_fraction(0.5);

    let row = DictionaryRow::new();
    ui.dictionaries()
        .insert(&row, ui.import_dictionary().index());
    row.set_title("NHK");
    row.set_subtitle("version");
    row.importing().set_visible(true);
    row.progress().set_visible(true);
    row.progress().set_fraction(0.25);

    let row = DictionaryRow::new();
    ui.dictionaries()
        .insert(&row, ui.import_dictionary().index());
    row.set_title("foo.zip");
    row.import_error().set_visible(true);

    {
        let ui = ui.clone();
        ui.import_dictionary().connect_activated(move |_| {
            glib::spawn_future_local(import_dictionaries(ui.clone(), window.clone()));
        });
    }
    ui.upcast()
}

#[expect(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]
async fn import_dictionaries(ui: Manager, window: gtk::Window) {
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
        let file = file.expect("list model should not be mutated during iteration");
        let file = file
            .downcast::<gio::File>()
            .expect("object should be a file");

        if let Err(err) = import_dictionary(&file).await {
            let title = if let Some(basename) = file.basename() {
                format!("Failed to import {basename:?}")
            } else if let Some(path) = file.path() {
                format!("Failed to import {path:?}")
            } else {
                format!("Failed to import dictionary")
            };

            let toast = adw::Toast::builder()
                .title(title)
                .button_label(gettext("Details"))
                .build();
            let window = window.clone();
            toast.connect_button_clicked(move |_| {
                let dialog = ErrorDialog::new();
                dialog.message().set_text(&format!("{err:?}"));
                adw::Dialog::builder()
                    .child(&dialog)
                    .build()
                    .present(Some(&window));
            });
            ui.toast_overlay().add_toast(toast);
        }
    }
}

#[expect(clippy::future_not_send, reason = "`gtk` types aren't `Send`")]
async fn import_dictionary(file: &gio::File) -> Result<()> {
    let (data, _) = file
        .load_bytes_future()
        .await
        .context("failed to load file into memory")?;
    println!("data = {}", data.len());
    Ok(())
}
