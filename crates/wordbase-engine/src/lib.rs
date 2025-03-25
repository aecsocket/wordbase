#![doc = include_str!("../README.md")]
#![allow(missing_docs, clippy::missing_errors_doc)]

mod db;
pub mod import;
pub mod lookup;
pub mod profile;
mod server;

use {
    anyhow::{Context, Result},
    futures::never::Never,
    import::Imports,
    lookup::Lookups,
    profile::Profiles,
    std::path::PathBuf,
    tokio::sync::{broadcast, mpsc},
    wordbase::{DictionaryState, ProfileState, protocol::ShowPopupRequest},
};

#[derive(Debug)]
#[non_exhaustive]
pub struct FullEngine {
    pub shared: Engine,
    pub recv_popup_request: mpsc::Receiver<ShowPopupRequest>,
}

#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Engine {
    pub profiles: Profiles,
    pub lookups: Lookups,
    pub imports: Imports,
    pub send_event: broadcast::Sender<Event>,
}

const CHANNEL_BUF_CAP: usize = 4;

#[derive(Debug, Clone)]
pub struct Config {
    pub db_path: PathBuf,
    pub max_db_connections: u32,
    pub max_concurrent_imports: usize,
}

pub async fn run(
    config: &Config,
) -> Result<(FullEngine, impl Future<Output = Result<Never>> + use<>)> {
    let db = db::setup(&config.db_path, config.max_db_connections)
        .await
        .context("failed to set up database")?;

    let (send_event, recv_event) = broadcast::channel(CHANNEL_BUF_CAP);
    let profiles = Profiles::new(db.clone(), send_event.clone());
    let lookups = Lookups::new(db.clone());
    let imports = Imports::new(db, config.max_concurrent_imports);

    let (send_popup_request, recv_popup_request) = mpsc::channel(CHANNEL_BUF_CAP);
    Ok((
        FullEngine {
            shared: Engine {
                profiles,
                lookups,
                imports,
                send_event,
            },
            recv_popup_request,
        },
        async { loop {} },
    ))
}

#[derive(Debug, Clone)]
pub enum Event {
    SyncProfiles(Vec<ProfileState>),
    SyncDictionaries(Vec<DictionaryState>),
}
