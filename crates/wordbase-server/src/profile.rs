use std::sync::Arc;

use poem::{Result, error::NotFoundError};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use wordbase::{Profile, ProfileConfig, ProfileId};
use wordbase_engine::Engine;

pub async fn index(engine: &Engine) -> Vec<Arc<Profile>> {
    engine.profiles().values().cloned().collect()
}

pub async fn find(engine: &Engine, profile_id: ProfileId) -> Result<Arc<Profile>> {
    Ok(engine
        .profiles()
        .get(&profile_id)
        .cloned()
        .ok_or(NotFoundError)?)
}

pub async fn delete(engine: &Engine, profile_id: ProfileId) -> Result<()> {
    engine.remove_profile(profile_id).await?;
    Ok(())
}

pub async fn add(engine: &Engine, req: Add) -> Result<AddResponse> {
    let new_profile_id = engine.add_profile(req.config).await?;
    Ok(AddResponse { new_profile_id })
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Add {
    pub config: ProfileConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct AddResponse {
    pub new_profile_id: ProfileId,
}

pub async fn copy(engine: &Engine, profile_id: ProfileId, req: Add) -> Result<AddResponse> {
    let new_profile_id = engine.copy_profile(profile_id, req.config).await?;
    Ok(AddResponse { new_profile_id })
}

pub async fn set_config(
    engine: &Engine,
    profile_id: ProfileId,
    config: ProfileConfig,
) -> Result<()> {
    engine.set_profile_config(profile_id, config).await?;
    Ok(())
}
