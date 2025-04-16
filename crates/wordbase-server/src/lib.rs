#![doc = include_str!("../README.md")]
#![allow(
    clippy::unused_async,
    reason = "OpenAPI endpoints must be async functions"
)]

mod lookup;

use {
    anyhow::Result,
    poem::listener::TcpListener,
    poem_openapi::{OpenApi, OpenApiService, payload::Json},
    tokio::net::ToSocketAddrs,
    wordbase_engine::Engine,
};

pub async fn run(engine: Engine, addr: impl ToSocketAddrs + Send) -> Result<()> {
    let service = OpenApiService::new(
        Api { engine },
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
    )
    .server("http://127.0.0.1:9518");
    let ui = service.swagger_ui();
    let app = poem::Route::new().nest("/", service).nest("/docs", ui);

    poem::Server::new(TcpListener::bind(addr)).run(app).await?;
    Ok(())
}

struct Api {
    engine: Engine,
}

#[OpenApi]
impl Api {
    #[oai(path = "/lookup/deinflect", method = "post")]
    async fn lookup_deinflect(
        &self,
        req: Json<lookup::DeinflectRequest>,
    ) -> lookup::DeinflectResponse {
        lookup::deinflect(&self.engine, req).await
    }

    #[oai(path = "/lookup/lemma", method = "post")]
    async fn lookup_lemma(&self, req: Json<lookup::LemmaRequest>) -> lookup::RecordsResponse {
        lookup::lemma(&self.engine, req).await
    }

    #[oai(path = "/lookup/expr", method = "post")]
    async fn lookup_expr(&self, req: Json<lookup::ExprRequest>) -> lookup::RecordsResponse {
        lookup::expr(&self.engine, req).await
    }
}
