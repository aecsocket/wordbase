use {
    axum::{
        http::StatusCode,
        response::{IntoResponse, Response},
    },
    derive_more::{Display, Error},
};

#[derive(Debug, Display, Error)]
pub enum AppError {
    #[display("internal error")]
    Internal(eyre::Report),
    #[display("request error")]
    Request(eyre::Report),
}

pub type Result<T, E = AppError> = std::result::Result<T, E>;

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            Self::Internal(err) => {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("{err:#?}")).into_response()
            }
            Self::Request(err) => (StatusCode::BAD_REQUEST, format!("{err:#?}")).into_response(),
        }
    }
}
