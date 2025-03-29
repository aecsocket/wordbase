use crate::{
    VERSION,
    request::{Request, RequestWrapper},
};

#[derive(Debug, Clone)]
pub struct Client {
    pub client: reqwest::Client,
    pub url: String,
    pub api_key: Option<String>,
}

impl Client {
    pub async fn send<R: Request>(&self, request: &R) -> reqwest::Result<R::Response> {
        let mut builder = self.client.get(&self.url).json(&RequestWrapper {
            version: VERSION,
            action: R::ACTION,
            params: if R::HAS_PARAMS { Some(request) } else { None },
        });
        if let Some(api_key) = &self.api_key {
            builder = builder.bearer_auth(api_key);
        }
        builder
            .send()
            .await?
            .error_for_status()?
            .json::<R::Response>()
            .await
    }
}
