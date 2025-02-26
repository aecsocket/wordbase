use anyhow::{Context as _, Result};
use gdk::prelude::{DeviceExt, DisplayExt, SeatExt};
use tokio::sync::mpsc;
use tracing::info;

use crate::exstatic::NewSentence;

pub async fn run(mut recv_new_sentence: mpsc::Receiver<NewSentence>) -> Result<()> {
    loop {
        let new_sentence = recv_new_sentence
            .recv()
            .await
            .context("new sentence channel closed")?;

        let display = gdk::Display::default().context("no display found")?;
        let kb = display.default_seat().unwrap().keyboard().unwrap();
        let (dev, x, y) = kb.surface_at_position();

        info!("dev = {dev:?} / {x}, {y}");
    }
}
