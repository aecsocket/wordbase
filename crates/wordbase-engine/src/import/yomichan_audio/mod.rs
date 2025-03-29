use std::io::{Cursor, Read};

use anyhow::{Context, Result, bail};
use bytes::Bytes;
use futures::future::BoxFuture;
use sqlx::{Sqlite, Transaction};
use tokio::sync::oneshot;
use wordbase::{Term, format};
use xz2::read::XzDecoder;

use crate::Engine;

use super::{ImportError, ImportTracker, Importer, insert_term};

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
    let mut archive = tar::Archive::new(XzDecoder::new(Cursor::new(&archive)));
    let mut scratch = Vec::new();
    for entry in archive
        .entries()
        .context("failed to read archive entries")?
    {
        let mut entry = entry.context("failed to read archive entry")?;
        let path = entry.path().context("failed to read entry path")?;
        let path = path
            .to_str()
            .with_context(|| format!("path {path:?} is not UTF-8"))?;
        if let Some(path) = path.strip_prefix(FORVO_PATH) {
            let path = path.to_owned();
            import_forvo(todo!(), &mut scratch, &path, &mut entry);
        }
    }

    todo!()
}

async fn import_forvo<R: Read>(
    tx: &mut Transaction<'_, Sqlite>,
    scratch: &mut Vec<u8>,
    path: &str,
    entry: &mut tar::Entry<'_, R>,
) -> Result<()> {
    let mut parts = path.split('/');
    let _username = parts.next().context("no Forvo username in path")?;
    let headword_file = parts.next().context("no headword in path")?;
    let headword = headword_file
        .rsplit_once('.')
        .map_or(headword_file, |(name, _)| name);

    let buf = blocking::unblock(move || {
        let mut buf = Vec::new();
        entry.read_to_end(&mut buf)?;
        anyhow::Ok(buf)
    })
    .await
    .context("failed to read file into memory")?;

    insert_term(tx, todo!(), &Term::new(headword), todo!(), scratch)
        .await
        .context("failed to insert term")?;

    Ok(())
}
