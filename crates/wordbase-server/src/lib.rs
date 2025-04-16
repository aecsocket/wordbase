#![doc = include_str!("../README.md")]
#![allow(
    clippy::unused_async,
    reason = "OpenAPI endpoints must be async functions"
)]

mod dict;
mod lookup;

use {
    futures::{FutureExt, stream::BoxStream},
    poem::{EndpointExt, Result, http::StatusCode, listener::TcpListener, web::Path},
    poem_openapi::{
        OpenApi, OpenApiService,
        payload::{EventStream, Json},
    },
    std::sync::Arc,
    tokio::net::ToSocketAddrs,
    wordbase::{Dictionary, DictionaryId},
    wordbase_engine::{Engine, NotFound},
};

pub async fn run(engine: Engine, addr: impl ToSocketAddrs + Send) -> anyhow::Result<()> {
    let service = OpenApiService::new(
        Api { engine },
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )
    .server("http://127.0.0.1:9518");
    let ui = service.swagger_ui();
    let app = poem::Route::new()
        .nest("/", service)
        .nest("/docs", ui)
        .catch_error(|_: NotFound| async move { StatusCode::NOT_FOUND });

    poem::Server::new(TcpListener::bind(addr)).run(app).await?;
    Ok(())
}

struct Api {
    engine: Engine,
}

#[OpenApi]
impl Api {
    #[oai(path = "/lookup/expr", method = "post")]
    async fn lookup_expr(
        &self,
        req: Json<lookup::ExprRequest>,
    ) -> Result<Json<Vec<lookup::RecordLookup>>> {
        lookup::expr(&self.engine, &req).await.map(Json)
    }

    #[oai(path = "/lookup/lemma", method = "post")]
    async fn lookup_lemma(
        &self,
        req: Json<lookup::LemmaRequest>,
    ) -> Result<Json<Vec<lookup::RecordLookup>>> {
        lookup::lemma(&self.engine, &req).await.map(Json)
    }

    #[oai(path = "/lookup/deinflect", method = "post")]
    async fn lookup_deinflect(
        &self,
        req: Json<lookup::DeinflectRequest>,
    ) -> Json<Vec<lookup::Deinflection>> {
        Json(lookup::deinflect(&self.engine, &req).await)
    }

    #[oai(path = "/dict", method = "get")]
    async fn dict_index(&self) -> Json<Vec<Arc<Dictionary>>> {
        Json(dict::index(&self.engine).await)
    }

    #[oai(path = "/dict/:dict_id", method = "get")]
    async fn dict_find(&self, dict_id: Path<DictionaryId>) -> Result<Json<Arc<Dictionary>>> {
        dict::find(&self.engine, *dict_id).await.map(Json)
    }

    #[oai(path = "/dict/import", method = "post")]
    async fn dict_import(
        &self,
        req: dict::Import,
    ) -> EventStream<BoxStream<'static, dict::ImportEvent>> {
        dict::import(&self.engine, req).boxed().await
    }

    #[oai(path = "/dict/:dict_id/position", method = "post")]
    async fn dict_set_position(
        &self,
        dict_id: Path<DictionaryId>,
        req: Json<dict::SetPosition>,
    ) -> Result<()> {
        dict::set_position(&self.engine, *dict_id, &req).await
    }

    #[oai(path = "/dict/:dict_id/enable", method = "post")]
    async fn dict_enable(
        &self,
        dict_id: Path<DictionaryId>,
        req: Json<dict::ToggleEnable>,
    ) -> Result<()> {
        dict::enable(&self.engine, *dict_id, &req).await
    }

    #[oai(path = "/dict/:dict_id/disable", method = "post")]
    async fn dict_disable(
        &self,
        dict_id: Path<DictionaryId>,
        req: Json<dict::ToggleEnable>,
    ) -> Result<()> {
        dict::disable(&self.engine, *dict_id, &req).await
    }

    #[oai(path = "/dict/:dict_id", method = "delete")]
    async fn dict_delete(&self, dict_id: Path<DictionaryId>) -> Result<()> {
        dict::delete(&self.engine, *dict_id).await
    }
}
