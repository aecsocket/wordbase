//! Exposes [`Engine`] over an HTTP REST API server.
#![allow(clippy::unused_async, reason = "API endpoints are inherently async")]

use {
    axum::Json,
    eyre::{Context, eyre},
    serde::{Deserialize, Serialize},
    std::{fmt::Display, sync::Arc},
    tokio::net::{TcpListener, ToSocketAddrs},
    utoipa::ToSchema,
    utoipa_axum::{router::OpenApiRouter, routes},
    utoipa_swagger_ui::{Config, SwaggerUi},
    wordbase_engine::{deinflect::Deinflectors, storage::EngineStorage},
};

// mod anki; // TODO
// mod dictionary;
mod error;
mod lookup;
mod profile;

pub use error::*;

/// Default port for serving the HTTP server on.
pub const DEFAULT_PORT: u16 = 9518;

/// Runs the HTTP REST API server at the given address.
///
/// # Errors
///
/// Errors if there is an unrecoverable server error.
pub async fn serve(
    storage: impl Into<Arc<EngineStorage>>,
    deinflectors: impl Into<Arc<Deinflectors>>,
    bind_addr: impl ToSocketAddrs + Send + Display,
) -> eyre::Result<()> {
    let (openapi_router, openapi) = OpenApiRouter::new()
        .routes(routes!(health_check))
        .merge(lookup::routes())
        .merge(profile::routes())
        .with_state(App {
            storage: storage.into(),
            deinflectors: deinflectors.into(),
        })
        .split_for_parts();

    let app = openapi_router.merge(
        SwaggerUi::new("/docs")
            .config(Config::default().try_it_out_enabled(true))
            .url("/docs/openapi.json", openapi),
    );

    let addr_str = bind_addr.to_string();
    let listener = TcpListener::bind(bind_addr)
        .await
        .wrap_err_with(|| eyre!("failed to bind server to {addr_str}"))?;
    axum::serve(listener, app)
        .await
        .wrap_err_with(|| eyre!("failed to run server on {addr_str}"))?;
    Ok(())
}

#[derive(Debug, Clone)]
struct App {
    storage: Arc<EngineStorage>,
    deinflectors: Arc<Deinflectors>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
struct Health {
    healthy: bool,
}

/// Checks if the server is in a valid state to process requests.
#[axum::debug_handler]
#[utoipa::path(
    get,
    path = "/health",
    responses((status = OK, body = Health)),
)]
async fn health_check() -> Json<Health> {
    Json(Health { healthy: true })
}

// #[OpenApi]
// impl App {
//     #[oai(path = "/lookup/sentence", method = "post")]
//     async fn lookup_sentence(
//         &self,
//         req: Json<lookup::Sentence>,
//     ) -> Result<Json<Vec<lookup::RecordEntry>>> {
//         lookup::sentence(self, req.0).await.map(Json)
//     }

//     #[oai(path = "/lookup/lemma", method = "post")]
//     async fn lookup_lemma(
//         &self,
//         req: Json<lookup::Lemma>,
//     ) -> Result<Json<Vec<lookup::RecordEntry>>> {
//         lookup::lemma(self, req.0).await.map(Json)
//     }

//     #[oai(path = "/lookup/deinflect", method = "post")]
//     async fn lookup_deinflect(
//         &self,
//         req: Json<lookup::Deinflect>,
//     ) -> Json<Vec<lookup::Deinflection>> {
//         Json(lookup::deinflect(self, req.0).await)
//     }

//     // #[oai(path = "/profile", method = "get")]
//     // async fn profile_index(&self) -> Json<Vec<Arc<Profile>>> {
//     //     Json(profile::index(self).await)
//     // }

//     // #[oai(path = "/profile/:profile_id", method = "get")]
//     // async fn profile_find(&self, profile_id: Path<ProfileId>) ->
//     // Result<Json<Arc<Profile>>> {     profile::find(self,
//     // profile_id.0).await.map(Json) }

//     // #[oai(path = "/profile/:profile_id", method = "delete")]
//     // async fn profile_delete(&self, profile_id: Path<ProfileId>) ->
// Result<()> {     //     profile::delete(self, profile_id.0).await
//     // }

//     // #[oai(path = "/profile", method = "put")]
//     // async fn profile_add(&self, req: Json<profile::Add>) ->
//     // Result<Json<profile::AddResponse>> {     profile::add(self,
//     // req.0).await.map(Json) }

//     // #[oai(path = "/profile/:profile_id/copy", method = "post")]
//     // async fn profile_copy(
//     //     &self,
//     //     profile_id: Path<ProfileId>,
//     //     req: Json<profile::Add>,
//     // ) -> Result<Json<profile::AddResponse>> {
//     //     profile::copy(self, profile_id.0, req.0).await.map(Json)
//     // }

//     // #[oai(path = "/dictionary", method = "get")]
//     // async fn dictionary_index(&self) -> Json<Vec<Arc<Dictionary>>> {
//     //     Json(dictionary::index(self).await)
//     // }

//     // #[oai(path = "/dictionary/:dictionary_id", method = "get")]
//     // async fn dictionary_find(
//     //     &self,
//     //     dictionary_id: Path<DictionaryId>,
//     // ) -> Result<Json<Arc<Dictionary>>> {
//     //     dictionary::find(self, dictionary_id.0).await.map(Json)
//     // }

//     // #[oai(path = "/dictionary/:dictionary_id", method = "delete")]
//     // async fn dictionary_delete(&self, dictionary_id: Path<DictionaryId>)
// ->     // Result<()> {     dictionary::delete(self, dictionary_id.0).await
//     // }

//     // #[oai(path = "/dictionary/import", method = "post")]
//     // async fn dictionary_import(
//     //     &self,
//     //     req: dictionary::Import,
//     // ) -> EventStream<BoxStream<'static, dictionary::ImportEvent>> {
//     //     dictionary::import(self, req).await
//     // }

//     // #[oai(path = "/dictionary/position/swap", method = "post")]
//     // async fn dictionary_position_swap(&self, req:
// Json<dictionary::PositionSwap>)     // -> Result<()> {
// dictionary::position_swap(self, req.0).await     // }

//     // #[oai(path = "/dictionary/enable", method = "post")]
//     // async fn dictionary_enable(&self, req: Json<dictionary::ToggleEnable>)
// ->     // Result<()> {     dictionary::enable(self, req.0).await
//     // }

//     // #[oai(path = "/dictionary/disable", method = "post")]
//     // async fn dictionary_disable(&self, req:
// Json<dictionary::ToggleEnable>) ->     // Result<()> {
// dictionary::disable(self, req.0).await     // }
// }

// #[derive(Debug, Clone, Object)]
// struct Term {
//     headword: Option<String>,
//     reading: Option<String>,
// }

// impl From<wordbase_types::Term> for Term {
//     fn from(value: wordbase_types::Term) -> Self {
//         match value {
//             wordbase_types::Term::Headword(headword) => Self {
//                 headword: Some(headword.into()),
//                 reading: None,
//             },
//             wordbase_types::Term::Reading(reading) => Self {
//                 headword: None,
//                 reading: Some(reading.into()),
//             },
//             wordbase_types::Term::Full(headword, reading) => Self {
//                 headword: Some(headword.into()),
//                 reading: Some(reading.into()),
//             },
//         }
//     }
// }
