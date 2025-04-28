use {
    poem::{Result, error::NotFoundError},
    poem_openapi::Object,
    serde::{Deserialize, Serialize},
    std::sync::Arc,
    wordbase::{NormString, Profile, ProfileId},
    wordbase_engine::Engine,
};

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
    let new_profile_id = engine.add_profile(req.name).await?;
    Ok(AddResponse { new_profile_id })
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct Add {
    pub name: Option<NormString>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
pub struct AddResponse {
    pub new_profile_id: ProfileId,
}

pub async fn copy(engine: &Engine, profile_id: ProfileId, req: Add) -> Result<AddResponse> {
    let new_profile_id = engine.copy_profile(profile_id, req.name).await?;
    Ok(AddResponse { new_profile_id })
}
