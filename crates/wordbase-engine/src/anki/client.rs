use {
    super::request::{Request, RequestWrapper},
    anyhow::Result,
    derive_more::{Display, Error},
    serde::{Deserialize, Serialize},
};

pub const VERSION: u32 = 6;

#[derive(Debug, Clone)]
pub struct AnkiClient {
    pub http_client: reqwest::Client,
    pub url: String,
    pub api_key: String,
}

impl AnkiClient {
    pub async fn send<R: Request>(&self, request: &R) -> Result<R::Response> {
        Ok(self
            .http_client
            .post(&self.url)
            .json(&RequestWrapper {
                version: VERSION,
                action: R::ACTION,
                params: if R::HAS_PARAMS { Some(request) } else { None },
            })
            .bearer_auth(&self.api_key)
            .send()
            .await?
            .error_for_status()?
            .json::<Response<R::Response>>()
            .await?
            .into_result()?)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T> {
    pub result: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Display, Error)]
pub struct Error(#[error(ignore)] pub String);

impl<T> Response<T> {
    pub fn into_result(self) -> Result<T, Error> {
        match (self.result, self.error) {
            (Some(result), _) => Ok(result),
            (None, Some(err)) => Err(Error(err)),
            (None, None) => Err(Error("(no message)".into())),
        }
    }
}
