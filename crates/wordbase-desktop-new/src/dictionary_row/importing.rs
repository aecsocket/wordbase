use crate::{
    engine,
    util::{AppComponent, impl_component},
};
use adw::prelude::*;
use anyhow::{Context as _, Result};
use bytes::Bytes;
use relm4::prelude::*;
use tokio::sync::oneshot;

use super::ui;

#[derive(Debug)]
pub struct DictionaryRow;

#[derive(Debug)]
#[doc(hidden)]
pub enum Msg {}

impl_component!(DictionaryRow);

impl AppComponent for DictionaryRow {
    type Args = gio::File;
    type Msg = Msg;
    type Ui = ui::DictionaryRow;

    async fn init(
        file: Self::Args,
        ui: Self::Ui,
        sender: AsyncComponentSender<Self>,
    ) -> AsyncComponentParts<Self> {
        ui.imported().set_visible(false);
        ui.processing().set_visible(true);
        ui.import_error().set_visible(false);
        ui.progress().set_visible(true);

        sender.command(|out, shutdown| {
            shutdown
                .register(async move {
                    if let Err(err) = import(file).await {
                        // tODO
                    }
                })
                .drop_on_shutdown()
        });

        AsyncComponentParts {
            model: Self,
            widgets: (),
        }
    }
}

async fn import(archive: gio::File, sender: relm4::Sender<Msg>) -> Result<()> {
    let (archive, _) = archive
        .load_bytes_future()
        .await
        .context("failed to read file into memory")?;
    let archive = Bytes::from(archive.to_vec());

    let (send_tracker, recv_tracker) = oneshot::channel();
    tokio::spawn(engine().import_dictionary(archive, send_tracker));

    Ok(())
}
