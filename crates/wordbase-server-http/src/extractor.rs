use {
    crate::{App, AppError},
    axum::extract::FromRequestParts,
    eyre::eyre,
    std::sync::Arc,
    utoipa::{
        IntoParams, PartialSchema,
        openapi::{Required, path},
    },
    wordbase_engine::{error::Context, profiles::ProfileState},
    wordbase_types::ProfileId,
};

pub struct ExtractProfile(pub Arc<ProfileState>);

const PROFILE_ID_HEADER: &str = "X-Wordbase-Profile-Id";

impl FromRequestParts<App> for ExtractProfile {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        app: &App,
    ) -> Result<Self, Self::Rejection> {
        let profile_id = parts
            .headers
            .get(PROFILE_ID_HEADER)
            .wrap_request_err_with(|| eyre!("`{PROFILE_ID_HEADER}` header is missing"))?
            .to_str()
            .wrap_request_err_with(|| eyre!("`{PROFILE_ID_HEADER}` header contains invalid ID"))?
            .parse::<ProfileId>()
            .wrap_request_err_with(|| eyre!("`{PROFILE_ID_HEADER}` header contains invalid ID"))?;
        let profile = app.storage.get_profile(profile_id)?;
        Ok(Self(profile))
    }
}

impl IntoParams for ExtractProfile {
    fn into_params(_: impl Fn() -> Option<path::ParameterIn>) -> Vec<path::Parameter> {
        vec![
            utoipa::openapi::path::ParameterBuilder::new()
                .name(PROFILE_ID_HEADER)
                .required(Required::True)
                .parameter_in(path::ParameterIn::Header)
                .description(Some("Profile ID for this operation"))
                .schema(Some(ProfileId::schema()))
                .build(),
        ]
    }
}
