use {
    crate::{App, Result},
    axum::{
        Json,
        extract::{Path, State},
    },
    serde::{Deserialize, Serialize},
    utoipa::ToSchema,
    utoipa_axum::{router::OpenApiRouter, routes},
    wordbase_types::{Profile, ProfileId},
};

pub fn routes() -> OpenApiRouter<App> {
    OpenApiRouter::new()
        .routes(routes!(get_all))
        .routes(routes!(get))
        .routes(routes!(create))
        .routes(routes!(delete))
}

/// Get all profiles.
#[axum::debug_handler]
#[utoipa::path(
    get,
    path = "/profile",
    responses((status = OK, body = Vec<Profile>))
)]
async fn get_all(State(app): State<App>) -> Json<Vec<Profile>> {
    let profiles = app
        .storage
        .profiles()
        .values()
        .map(|profile| profile.to_profile())
        .collect();
    Json(profiles)
}

/// Get a profile by its ID.
#[axum::debug_handler]
#[utoipa::path(
    get,
    path = "/profile/{id}",
    params(("id", description = "Profile ID")),
    responses((status = OK, body = Vec<Profile>))
)]
async fn get(State(app): State<App>, Path((id,)): Path<(ProfileId,)>) -> Result<Json<Profile>> {
    let profile = app.storage.get_profile(id)?.to_profile();
    Ok(Json(profile))
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(examples(example_create))]
struct Create {
    /// New profile name.
    name: String,
}

fn example_create() -> Create {
    Create {
        name: "German Sentence Mining".into(),
    }
}

/// Create a profile.
#[axum::debug_handler]
#[utoipa::path(
    put,
    path = "/profile",
    request_body = inline(Create),
    responses((status = OK, body = Vec<Profile>))
)]
async fn create(State(app): State<App>, Json(req): Json<Create>) -> Result<Json<ProfileId>> {
    let profile_id = app.storage.create_profile(&req.name).await?;
    Ok(Json(profile_id))
}

/// Delete a profile.
#[axum::debug_handler]
#[utoipa::path(
    delete,
    path = "/profile/{id}",
    params(("id", description = "Profile ID")),
    responses((status = OK))
)]
async fn delete(State(app): State<App>, Path((id,)): Path<(ProfileId,)>) -> Result<()> {
    app.storage.remove_profile(id).await?;
    Ok(())
}
