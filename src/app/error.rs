use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

use crate::{services::error::ServiceError, sessions::error::SessionError};

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Service error: {0}")]
    Service(#[from] ServiceError),

    #[error("credenciales invalidas")]
    CredencialesInvalidas,

    #[error("session error: {0}")]
    Session(#[from] SessionError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        eprintln!("[ApiError] {self}");

        let (status, mensaje) = match self {
            ApiError::CredencialesInvalidas => {
                (StatusCode::UNAUTHORIZED, "credenciales inválidas")
            }
            ApiError::Service(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "error interno")
            }
            ApiError::Session(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "error interno")
            }
        };

        (status, mensaje).into_response()
    }
}