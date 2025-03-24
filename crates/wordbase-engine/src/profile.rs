use anyhow::{Context, Result};
use sqlx::{Pool, Sqlite};
use tokio::sync::broadcast;
use wordbase::{ProfileId, ProfileMeta, ProfileState, protocol::NotFound};

use crate::{Event, db};

#[derive(Debug, Clone)]
pub struct Profiles {
    db: Pool<Sqlite>,
    send_event: broadcast::Sender<Event>,
}

impl Profiles {
    pub(super) const fn new(db: Pool<Sqlite>, send_event: broadcast::Sender<Event>) -> Self {
        Self { db, send_event }
    }

    pub async fn create(&self, meta: &ProfileMeta) -> Result<ProfileId> {
        let result = db::profile::create(&self.db, meta).await?;
        let profiles = self.all().await.context("failed to fetch all profiles")?;
        _ = self.send_event.send(Event::SyncProfiles(profiles));
        Ok(result)
    }

    pub async fn current_id(&self) -> Result<ProfileId> {
        db::profile::current_id(&self.db).await
    }

    pub async fn all(&self) -> Result<Vec<ProfileState>> {
        db::profile::all(&self.db).await
    }

    pub async fn remove(&self, profile_id: ProfileId) -> Result<Result<(), NotFound>> {
        db::profile::remove(&self.db, profile_id).await
    }
}
