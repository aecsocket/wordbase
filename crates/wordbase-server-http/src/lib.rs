//! Exposes [`Engine`] over an HTTP REST API server.
#![allow(clippy::unused_async, reason = "API endpoints are inherently async")]

use {
    axum::{
        Json,
        http::StatusCode,
        response::{IntoResponse, Response},
    },
    derive_more::From,
    eyre::{Context, eyre},
    serde::{Deserialize, Serialize},
    std::{fmt::Display, sync::Arc},
    tokio::net::{TcpListener, ToSocketAddrs},
    utoipa::{OpenApi, ToSchema},
    utoipa_axum::{router::OpenApiRouter, routes},
    utoipa_swagger_ui::{Config, SwaggerUi},
    wordbase_engine::{deinflect::Deinflectors, storage::EngineStorage},
};

// mod anki; // TODO
mod dictionary;
mod extractor;
mod lookup;
mod profile;

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
        .merge(dictionary::routes())
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
            .url("/docs/openapi.json", ApiDoc::openapi().merge_from(openapi)),
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

#[expect(clippy::needless_for_each, reason = "due to `OpenApi` generated code")]
mod api_doc {
    #[derive(utoipa::OpenApi)]
    #[openapi(info(title = "wordbase"))]
    #[doc(hidden)]
    pub struct ApiDoc;
}

pub use api_doc::ApiDoc;

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
    responses((status = OK, body = inline(Health))),
)]
async fn health_check() -> Json<Health> {
    Json(Health { healthy: true })
}

#[derive(Debug, From)]
struct AppError(pub wordbase_engine::Error);

type Result<T, E = AppError> = std::result::Result<T, E>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self(wordbase_engine::Error::Internal(err)) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#?}")).into_response()
            }
            Self(wordbase_engine::Error::Request(err)) => {
                (StatusCode::BAD_REQUEST, format!("{err:#?}")).into_response()
            }
        }
    }
}
