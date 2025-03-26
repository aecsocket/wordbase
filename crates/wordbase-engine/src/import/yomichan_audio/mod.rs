use std::io::Cursor;

use anyhow::{Context, Result, bail};
use bytes::Bytes;
use futures::future::BoxFuture;
use tokio::sync::oneshot;
use xz2::read::XzDecoder;

use crate::Engine;

use super::{ImportError, ImportTracker, Importer};

pub struct YomichanAudio;

impl Importer for YomichanAudio {
    fn validate(&self, archive: Bytes) -> BoxFuture<'_, Result<()>> {
        Box::pin(blocking::unblock(move || validate_blocking(&archive)))
    }

    fn import<'a>(
        &'a self,
        engine: &'a Engine,
        archive: Bytes,
        send_tracker: oneshot::Sender<ImportTracker>,
    ) -> BoxFuture<'a, Result<(), ImportError>> {
        todo!();
    }
}

const FORVO_PATH: &str = "user_files/forvo_files/";
const JPOD_PATH: &str = "user_files/jpod_files/";
const NHK16_PATH: &str = "user_files/nhk16_files/";
const SHINMEIKAI8_PATH: &str = "user_files/shinmeikai8_files/";
const MARKER_PATHS: &[&str] = &[FORVO_PATH, JPOD_PATH, NHK16_PATH, SHINMEIKAI8_PATH];

fn validate_blocking(archive: &[u8]) -> Result<()> {
    let mut archive = tar::Archive::new(XzDecoder::new(Cursor::new(archive)));
    for entry in archive
        .entries()
        .context("failed to read archive entries")?
    {
        let Ok(entry) = entry else {
            continue;
        };
        let Ok(path) = entry.path() else {
            continue;
        };
        let Some(path) = path.to_str() else {
            continue;
        };
        if MARKER_PATHS.contains(&path) {
            return Ok(());
        }
    }
    bail!("missing one of {MARKER_PATHS:?}");
}

async fn import(
    engine: &Engine,
    archive: Bytes,
    send_tracker: oneshot::Sender<ImportTracker>,
) -> Result<(), ImportError> {
    todo!()
}
