use {
    crate::App,
    axum::{Json, extract::State},
    utoipa_axum::{router::UtoipaMethodRouter, routes},
    wordbase_types::Profile,
};

pub fn routes() -> UtoipaMethodRouter<App> {
    routes!(get_all)
}

#[axum::debug_handler]
#[utoipa::path(get, path = "/profile", responses((status = OK, body = Vec<Profile>)))]
async fn get_all(State(app): State<App>) -> Json<Vec<Profile>> {
    Json(
        app.storage
            .profiles()
            .iter()
            .map(|(&id, profile)| Profile {
                id,
                name: profile.name.clone(),
                sorting_dictionary: profile.sorting_dictionary,
                enabled_dictionaries: profile.enabled_dictionaries.iter().copied().collect(),
            })
            .collect(),
    )
}

// pub async fn find(app: &App, profile_id: ProfileId) -> Result<Arc<Profile>> {
//     Ok(app
//         .engine
//         .profiles()
//         .get(&profile_id)
//         .cloned()
//         .ok_or(NotFoundError)?)
// }

// pub async fn delete(app: &App, profile_id: ProfileId) -> Result<()> {
//     app.engine.remove_profile(profile_id).await?;
//     Ok(())
// }

// pub async fn add(app: &App, req: Add) -> Result<AddResponse> {
//     let new_profile_id = app.engine.add_profile(req.name).await?;
//     Ok(AddResponse { new_profile_id })
// }

// #[derive(Debug, Clone, Object)]
// pub struct Add {
//     pub name: Option<NormString>,
// }

// #[derive(Debug, Clone, Object)]
// pub struct AddResponse {
//     pub new_profile_id: ProfileId,
// }

// pub async fn copy(app: &App, profile_id: ProfileId, req: Add) ->
// Result<AddResponse> {     let new_profile_id =
// app.engine.copy_profile(profile_id, req.name).await?;     Ok(AddResponse {
// new_profile_id }) }
