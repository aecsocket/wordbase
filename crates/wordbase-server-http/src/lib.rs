//! Exposes [`Engine`] over an HTTP REST API server.
#![allow(clippy::unused_async, reason = "API endpoints are inherently async")]

use {
    eyre::Context,
    futures::stream::BoxStream,
    poem::{
        EndpointExt as _, Response, Result, http::StatusCode, listener::TcpListener, web::Path,
    },
    poem_openapi::{
        Object, OpenApi, OpenApiService,
        payload::{EventStream, Json},
    },
    std::{fmt::Display, sync::Arc},
    tokio::net::ToSocketAddrs,
    wordbase_engine::{deinflect::Deinflectors, storage::EngineStorage},
};

// mod anki; // TODO
// mod dictionary;
mod lookup;
// mod profile;

/// Default port for serving the HTTP server on.
pub const DEFAULT_PORT: u16 = 9518;

/// Runs the HTTP REST API server at the given address.
///
/// # Errors
///
/// Errors if there is an unrecoverable server error.
pub async fn serve(
    storage: EngineStorage,
    deinflectors: Deinflectors,
    addr: impl ToSocketAddrs + Send + Display,
) -> eyre::Result<()> {
    let addr_str = addr.to_string();
    let v1 = OpenApiService::new(
        App {
            storage,
            deinflectors,
        },
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )
    .server(format!("http://{addr_str}/api/v1"));
    let docs = v1.swagger_ui();
    let spec_json = v1.spec_endpoint();
    let spec_yaml = v1.spec_endpoint_yaml();
    let app = poem::Route::new()
        .nest("/api/v1", v1)
        .nest("/api/docs", docs)
        .at("/api/spec.json", spec_json)
        .at("/api/spec.yaml", spec_yaml);
    // .catch_error(|_: NotFound| async move {
    //     Response::builder()
    //         .status(StatusCode::NOT_FOUND)
    //         .body("not found")
    // });

    poem::Server::new(TcpListener::bind(addr))
        .run(app)
        .await
        .wrap_err_with(|| format!("failed to run server on {addr_str}"))
}

struct App {
    storage: EngineStorage,
    deinflectors: Deinflectors,
}

#[OpenApi]
impl App {
    #[oai(path = "/lookup/sentence", method = "post")]
    async fn lookup_sentence(
        &self,
        req: Json<lookup::Sentence>,
    ) -> Result<Json<Vec<lookup::RecordEntry>>> {
        lookup::sentence(self, req.0).await.map(Json)
    }

    #[oai(path = "/lookup/lemma", method = "post")]
    async fn lookup_lemma(
        &self,
        req: Json<lookup::Lemma>,
    ) -> Result<Json<Vec<lookup::RecordEntry>>> {
        lookup::lemma(self, req.0).await.map(Json)
    }

    #[oai(path = "/lookup/deinflect", method = "post")]
    async fn lookup_deinflect(
        &self,
        req: Json<lookup::Deinflect>,
    ) -> Json<Vec<lookup::Deinflection>> {
        Json(lookup::deinflect(self, req.0).await)
    }

    // #[oai(path = "/profile", method = "get")]
    // async fn profile_index(&self) -> Json<Vec<Arc<Profile>>> {
    //     Json(profile::index(self).await)
    // }

    // #[oai(path = "/profile/:profile_id", method = "get")]
    // async fn profile_find(&self, profile_id: Path<ProfileId>) ->
    // Result<Json<Arc<Profile>>> {     profile::find(self,
    // profile_id.0).await.map(Json) }

    // #[oai(path = "/profile/:profile_id", method = "delete")]
    // async fn profile_delete(&self, profile_id: Path<ProfileId>) -> Result<()> {
    //     profile::delete(self, profile_id.0).await
    // }

    // #[oai(path = "/profile", method = "put")]
    // async fn profile_add(&self, req: Json<profile::Add>) ->
    // Result<Json<profile::AddResponse>> {     profile::add(self,
    // req.0).await.map(Json) }

    // #[oai(path = "/profile/:profile_id/copy", method = "post")]
    // async fn profile_copy(
    //     &self,
    //     profile_id: Path<ProfileId>,
    //     req: Json<profile::Add>,
    // ) -> Result<Json<profile::AddResponse>> {
    //     profile::copy(self, profile_id.0, req.0).await.map(Json)
    // }

    // #[oai(path = "/dictionary", method = "get")]
    // async fn dictionary_index(&self) -> Json<Vec<Arc<Dictionary>>> {
    //     Json(dictionary::index(self).await)
    // }

    // #[oai(path = "/dictionary/:dictionary_id", method = "get")]
    // async fn dictionary_find(
    //     &self,
    //     dictionary_id: Path<DictionaryId>,
    // ) -> Result<Json<Arc<Dictionary>>> {
    //     dictionary::find(self, dictionary_id.0).await.map(Json)
    // }

    // #[oai(path = "/dictionary/:dictionary_id", method = "delete")]
    // async fn dictionary_delete(&self, dictionary_id: Path<DictionaryId>) ->
    // Result<()> {     dictionary::delete(self, dictionary_id.0).await
    // }

    // #[oai(path = "/dictionary/import", method = "post")]
    // async fn dictionary_import(
    //     &self,
    //     req: dictionary::Import,
    // ) -> EventStream<BoxStream<'static, dictionary::ImportEvent>> {
    //     dictionary::import(self, req).await
    // }

    // #[oai(path = "/dictionary/position/swap", method = "post")]
    // async fn dictionary_position_swap(&self, req: Json<dictionary::PositionSwap>)
    // -> Result<()> {     dictionary::position_swap(self, req.0).await
    // }

    // #[oai(path = "/dictionary/enable", method = "post")]
    // async fn dictionary_enable(&self, req: Json<dictionary::ToggleEnable>) ->
    // Result<()> {     dictionary::enable(self, req.0).await
    // }

    // #[oai(path = "/dictionary/disable", method = "post")]
    // async fn dictionary_disable(&self, req: Json<dictionary::ToggleEnable>) ->
    // Result<()> {     dictionary::disable(self, req.0).await
    // }
}

#[derive(Debug, Clone, Object)]
struct Term {
    headword: Option<String>,
    reading: Option<String>,
}

impl From<wordbase_types::Term> for Term {
    fn from(value: wordbase_types::Term) -> Self {
        match value {
            wordbase_types::Term::Headword(headword) => Self {
                headword: Some(headword.into()),
                reading: None,
            },
            wordbase_types::Term::Reading(reading) => Self {
                headword: None,
                reading: Some(reading.into()),
            },
            wordbase_types::Term::Full(headword, reading) => Self {
                headword: Some(headword.into()),
                reading: Some(reading.into()),
            },
        }
    }
}
