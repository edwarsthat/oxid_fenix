use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

use crate::{
    routes::protocol::WsResponse, services::error::ServiceError, sessions::error::SessionError,
};

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
            ApiError::CredencialesInvalidas => (StatusCode::UNAUTHORIZED, "credenciales inválidas"),
            ApiError::Service(_) => (StatusCode::INTERNAL_SERVER_ERROR, "error interno"),
            ApiError::Session(_) => (StatusCode::INTERNAL_SERVER_ERROR, "error interno"),
        };

        (status, mensaje).into_response()
    }
}

mod code {
    pub const UNAUTHORIZED: u16 = 401;
    pub const INTERNAL: u16 = 500;
}

#[derive(Debug, Error)]
pub enum WsError {
    #[error("token ausente")]
    TokenAusente,

    #[error("token invalido")]
    TokenInvalido,

    #[error("sesión expirada")]
    SesionExpirada,

    #[error("sesión error: {0}")]
    Session(#[from] SessionError),
}

impl WsError {
    pub fn status_code(&self) -> u16 {
        match self {
            WsError::TokenAusente | WsError::TokenInvalido | WsError::SesionExpirada => {
                code::UNAUTHORIZED
            }
            WsError::Session(e) => Self::session_status_code(e),
        }
    }

    pub fn into_ws_response(self, id: impl Into<String>) -> WsResponse {
        let code = self.status_code();
        if code >= code::INTERNAL {
            tracing::error!(error = %self, "error interno en webSocket");
        } else {
            tracing::debug!(error = %self, "rechazo de autenticacion en WebSocket")
        }

        WsResponse::error(id, code as i32, Self::client_message(code))
    }

    pub fn clode_code(&self) -> u16 {
        match self.status_code() {
            code::UNAUTHORIZED => 1008,
            _ => 1011,
        }
    }

    fn client_message(code: u16) -> &'static str {
        match code {
            code::UNAUTHORIZED => "no autorizado",
            _ => "error interno",
        }
    }

    fn session_status_code(e: &SessionError) -> u16 {
        match e {
            SessionError::Expired
            | SessionError::Revoked
            | SessionError::NotFound => {
                code::UNAUTHORIZED
            }

            _ => code::INTERNAL,
        }
    }
}

impl IntoResponse for WsError {
    fn into_response(self) -> axum::response::Response {
        eprintln!("[WsError] {self}");

        let code = self.status_code();
        let status = StatusCode::from_u16(code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

        (status, Self::client_message(code)).into_response()
    }
}
