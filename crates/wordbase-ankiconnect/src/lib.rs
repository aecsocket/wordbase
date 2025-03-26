#![doc = include_str!("../README.md")]

use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response<T> {
    pub result: Option<T>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Display, Error)]
pub struct Error(pub String);

impl<T> Response<T> {
    pub fn into_result(self) -> Result<T, Error> {
        match (self.result, self.error) {
            (Some(result), _) => Ok(result),
            (None, Some(err)) => Err(Error(err)),
            (None, None) => Err(Error("(no message)".into())),
        }
    }
}
