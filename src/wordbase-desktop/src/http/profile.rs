use {
    crate::http::App,
    poem::{Result, error::NotFoundError},
    poem_openapi::Object,
    std::sync::Arc,
    wordbase::{NormString, Profile, ProfileId},
};

pub async fn index(app: &App) -> Vec<Arc<Profile>> {
    app.engine.profiles().values().cloned().collect()
}

pub async fn find(app: &App, profile_id: ProfileId) -> Result<Arc<Profile>> {
    Ok(app
        .engine
        .profiles()
        .get(&profile_id)
        .cloned()
        .ok_or(NotFoundError)?)
}

pub async fn delete(app: &App, profile_id: ProfileId) -> Result<()> {
    app.engine.remove_profile(profile_id).await?;
    Ok(())
}

pub async fn add(app: &App, req: Add) -> Result<AddResponse> {
    let new_profile_id = app.engine.add_profile(req.name).await?;
    Ok(AddResponse { new_profile_id })
}

#[derive(Debug, Clone, Object)]
pub struct Add {
    pub name: Option<NormString>,
}

#[derive(Debug, Clone, Object)]
pub struct AddResponse {
    pub new_profile_id: ProfileId,
}

pub async fn copy(app: &App, profile_id: ProfileId, req: Add) -> Result<AddResponse> {
    let new_profile_id = app.engine.copy_profile(profile_id, req.name).await?;
    Ok(AddResponse { new_profile_id })
}
