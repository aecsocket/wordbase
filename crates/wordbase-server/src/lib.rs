#![doc = include_str!("../README.md")]
#![allow(
    clippy::unused_async,
    reason = "OpenAPI endpoints must be async functions"
)]

mod anki;
mod dict;
mod lookup;
mod profile;

use {
    anyhow::Context,
    futures::stream::BoxStream,
    poem::{EndpointExt, Response, Result, http::StatusCode, listener::TcpListener, web::Path},
    poem_openapi::{
        Object, OpenApi, OpenApiService,
        payload::{EventStream, Json},
    },
    serde::{Deserialize, Serialize},
    std::{fmt::Display, sync::Arc},
    tokio::net::ToSocketAddrs,
    wordbase::{Dictionary, DictionaryId, NormString, Profile, ProfileConfig, ProfileId},
    wordbase_engine::{Engine, NotFound},
};

/// Runs the HTTP server.
///
/// # Errors
///
/// Errors if there is an unrecoverable server error.
pub async fn run(engine: Engine, addr: impl ToSocketAddrs + Send + Display) -> anyhow::Result<()> {
    let service = OpenApiService::new(
        V1 { engine },
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )
    .server(format!("http://{addr}/api/v1"));
    let ui = service.swagger_ui();
    let spec = service.spec_endpoint();
    let spec_yaml = service.spec_endpoint_yaml();
    let app = poem::Route::new()
        .nest("/api/v1", service)
        .nest("/api/docs", ui)
        .at("/api/spec.json", spec)
        .at("/api/spec.yaml", spec_yaml)
        .catch_error(|_: NotFound| async move {
            Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body("not found")
        });

    poem::Server::new(TcpListener::bind(addr)).run(app).await?;
    Ok(())
}

/// Default port for serving the HTTP server on.
pub const HTTP_PORT: u16 = 9518;

struct V1 {
    engine: Engine,
}

#[OpenApi]
impl V1 {
    #[oai(path = "/profile", method = "get")]
    async fn profile_index(&self) -> Json<Vec<Arc<Profile>>> {
        Json(profile::index(&self.engine).await)
    }

    #[oai(path = "/profile/:profile_id", method = "get")]
    async fn profile_find(&self, profile_id: Path<ProfileId>) -> Result<Json<Arc<Profile>>> {
        profile::find(&self.engine, profile_id.0).await.map(Json)
    }

    #[oai(path = "/profile/:profile_id", method = "delete")]
    async fn profile_delete(&self, profile_id: Path<ProfileId>) -> Result<()> {
        profile::delete(&self.engine, profile_id.0).await
    }

    #[oai(path = "/profile", method = "put")]
    async fn profile_add(&self, req: Json<profile::Add>) -> Result<Json<profile::AddResponse>> {
        profile::add(&self.engine, req.0).await.map(Json)
    }

    #[oai(path = "/profile/:profile_id/copy", method = "post")]
    async fn profile_copy(
        &self,
        profile_id: Path<ProfileId>,
        req: Json<profile::Add>,
    ) -> Result<Json<profile::AddResponse>> {
        profile::copy(&self.engine, profile_id.0, req.0)
            .await
            .map(Json)
    }

    #[oai(path = "/profile/:profile_id/config", method = "post")]
    async fn profile_set_config(
        &self,
        profile_id: Path<ProfileId>,
        req: Json<ProfileConfig>,
    ) -> Result<()> {
        profile::set_config(&self.engine, profile_id.0, req.0).await
    }

    #[oai(path = "/lookup/expr", method = "post")]
    async fn lookup_expr(
        &self,
        req: Json<lookup::ExprRequest>,
    ) -> Result<Json<Vec<lookup::RecordLookup>>> {
        lookup::expr(&self.engine, req.0).await.map(Json)
    }

    #[oai(path = "/lookup/lemma", method = "post")]
    async fn lookup_lemma(
        &self,
        req: Json<lookup::Lemma>,
    ) -> Result<Json<Vec<lookup::RecordLookup>>> {
        lookup::lemma(&self.engine, req.0).await.map(Json)
    }

    #[oai(path = "/lookup/deinflect", method = "post")]
    async fn lookup_deinflect(
        &self,
        req: Json<lookup::Deinflect>,
    ) -> Json<Vec<lookup::Deinflection>> {
        Json(lookup::deinflect(&self.engine, req.0).await)
    }

    #[oai(path = "/dictionary", method = "get")]
    async fn dictionary_index(&self) -> Json<Vec<Arc<Dictionary>>> {
        Json(dict::index(&self.engine).await)
    }

    #[oai(path = "/dictionary/:dictionary_id", method = "get")]
    async fn dictionary_find(
        &self,
        dictionary_id: Path<DictionaryId>,
    ) -> Result<Json<Arc<Dictionary>>> {
        dict::find(&self.engine, dictionary_id.0).await.map(Json)
    }

    #[oai(path = "/dictionary/:dictionary_id", method = "delete")]
    async fn dictionary_delete(&self, dictionary_id: Path<DictionaryId>) -> Result<()> {
        dict::delete(&self.engine, dictionary_id.0).await
    }

    #[oai(path = "/dictionary/import", method = "post")]
    async fn dictionary_import(
        &self,
        req: dict::Import,
    ) -> EventStream<BoxStream<'static, dict::ImportEvent>> {
        dict::import(&self.engine, req).await
    }

    #[oai(path = "/dictionary/:dictionary_id/position", method = "post")]
    async fn dictionary_set_position(
        &self,
        dictionary_id: Path<DictionaryId>,
        req: Json<dict::SetPosition>,
    ) -> Result<()> {
        dict::set_position(&self.engine, dictionary_id.0, req.0).await
    }

    #[oai(path = "/dictionary/:dictionary_id/enable", method = "post")]
    async fn dictionary_enable(
        &self,
        dictionary_id: Path<DictionaryId>,
        req: Json<dict::ToggleEnable>,
    ) -> Result<()> {
        dict::enable(&self.engine, dictionary_id.0, req.0).await
    }

    #[oai(path = "/dictionary/:dictionary_id/disable", method = "post")]
    async fn dictionary_disable(
        &self,
        dictionary_id: Path<DictionaryId>,
        req: Json<dict::ToggleEnable>,
    ) -> Result<()> {
        dict::disable(&self.engine, dictionary_id.0, req.0).await
    }

    #[oai(path = "/anki/note", method = "put")]
    async fn anki_note_add(&self, req: Json<anki::NoteAdd>) -> Result<()> {
        anki::note_add(&self.engine, req.0).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Object)]
struct Term {
    headword: Option<NormString>,
    reading: Option<NormString>,
}

impl From<wordbase::Term> for Term {
    fn from(value: wordbase::Term) -> Self {
        Self {
            headword: value.headword().cloned(),
            reading: value.reading().cloned(),
        }
    }
}

impl TryFrom<Term> for wordbase::Term {
    type Error = anyhow::Error;

    fn try_from(value: Term) -> Result<Self, Self::Error> {
        Self::new(value.headword, value.reading)
            .context("must have at least one of headword or reading")
    }
}
