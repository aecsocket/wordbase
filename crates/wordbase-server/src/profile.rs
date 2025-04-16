use poem::{Result, error::NotFoundError};
use poem_openapi::Object;
use serde::{Deserialize, Serialize};
use wordbase::{Profile, ProfileId, ProfileMeta};
use wordbase_engine::{Engine, profile::ProfileState};

pub async fn index(engine: &Engine) -> Vec<Profile> {
    engine.profiles().values().map(|v| convert(v)).collect()
}

pub async fn find(engine: &Engine, profile_id: ProfileId) -> Result<Profile> {
    Ok(convert(
        engine.profiles().get(&profile_id).ok_or(NotFoundError)?,
    ))
}

pub async fn delete(engine: &Engine, profile_id: ProfileId) -> Result<()> {
    engine.remove_profile(profile_id).await?;
    Ok(())
}

pub async fn copy(engine: &Engine, req: &CopyRequest) -> Result<CopyResponse> {
    let new_profile_id = engine.copy_profile(req.source_id, &req.new_meta).await?;
    Ok(CopyResponse { new_profile_id })
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct CopyRequest {
    pub source_id: ProfileId,
    pub new_meta: ProfileMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct CopyResponse {
    pub new_profile_id: ProfileId,
}

pub fn convert(state: &ProfileState) -> Profile {
    Profile {
        id: state.id,
        meta: state.meta.clone(),
        enabled_dictionaries: state.enabled_dictionaries.clone(),
        sorting_dictionary: state.sorting_dictionary,
    }
}
