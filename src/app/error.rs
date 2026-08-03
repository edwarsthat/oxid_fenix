use axum::{http::StatusCode, response::IntoResponse};
use thiserror::Error;

use crate::{models::validations::ValidacionError, services::error::ServiceError};

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("Service error: {0}")]
    Service(#[from] ServiceError),

    #[error("credenciales invalidas")]
    CredencialesInvalidas,

    #[error("usuario inactivo")]
    UsuarioInactivo,

    #[error("token ausente")]
    TokenAusente,

    #[error("token invalido")]
    TokenInvalido,

    #[error("error de hash: {0}")]
    Hash(#[from] argon2::password_hash::Error),

    #[error("validacion: {0}")]
    Validacion(#[from] ValidacionError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        eprintln!("[ApiError] {self}");

        // `Validacion` es el único que devuelve su mensaje tal cual: son reglas
        // públicas que el usuario necesita ver para corregir. El resto sigue con
        // mensajes genéricos para no filtrar detalles internos.
        let (status, mensaje) = match self {
            ApiError::CredencialesInvalidas => (
                StatusCode::UNAUTHORIZED,
                "credenciales inválidas".to_string(),
            ),
            ApiError::UsuarioInactivo => (StatusCode::FORBIDDEN, "usuario inactivo".to_string()),
            ApiError::Service(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error interno".to_string(),
            ),
            ApiError::TokenAusente => (
                StatusCode::UNAUTHORIZED,
                "credenciales inválidas".to_string(),
            ),
            ApiError::TokenInvalido => (
                StatusCode::UNAUTHORIZED,
                "credenciales inválidas".to_string(),
            ),
            ApiError::Hash(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "error interno".to_string(),
            ),
            ApiError::Validacion(err) => (StatusCode::BAD_REQUEST, err.mensaje().to_string()),
        };

        (status, mensaje).into_response()
    }
}

mod code {
    pub const UNAUTHORIZED: u16 = 401;
}

#[derive(Debug, Error)]
pub enum WsError {
    #[error("token ausente")]
    TokenAusente,

    #[error("token invalido")]
    TokenInvalido,
}

impl WsError {
    pub fn status_code(&self) -> u16 {
        match self {
            WsError::TokenAusente | WsError::TokenInvalido => code::UNAUTHORIZED,
        }
    }

    fn client_message(code: u16) -> &'static str {
        match code {
            code::UNAUTHORIZED => "no autorizado",
            _ => "error interno",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_code_errores_de_autenticacion() {
        // Los errores de auth directos deben devolver 401
        assert_eq!(WsError::TokenAusente.status_code(), 401);
        assert_eq!(WsError::TokenInvalido.status_code(), 401);
    }

    #[test]
    fn client_message_por_codigo() {
        // el codigo 401 devuelve el mensaje de "no autorizado"
        assert_eq!(WsError::client_message(401), "no autorizado");

        // cualquier otro codigo cae en el mensaje generico
        assert_eq!(WsError::client_message(500), "error interno");
        assert_eq!(WsError::client_message(418), "error interno");
    }

    #[test]
    fn ws_error_into_response_http_status() {
        assert_eq!(
            WsError::TokenAusente.into_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            WsError::TokenInvalido.into_response().status(),
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn api_error_into_response_http_status() {
        // credenciales invalidas -> 401
        assert_eq!(
            ApiError::CredencialesInvalidas.into_response().status(),
            StatusCode::UNAUTHORIZED
        );

        // usuario inactivo -> 403
        assert_eq!(
            ApiError::UsuarioInactivo.into_response().status(),
            StatusCode::FORBIDDEN
        );

        // error de servicio (base de datos) -> 500
        assert_eq!(
            ApiError::Service(ServiceError::Database(sqlx::Error::RowNotFound))
                .into_response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        // validacion -> 400, y este sí propaga su mensaje
        let resp = ApiError::Validacion(ValidacionError::nuevo("email no valido")).into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
