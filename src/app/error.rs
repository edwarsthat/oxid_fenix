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

    #[error("sesión error: {0}")]
    Session(#[from] SessionError),
}

impl WsError {
    pub fn status_code(&self) -> u16 {
        match self {
            WsError::TokenAusente | WsError::TokenInvalido => code::UNAUTHORIZED,
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
            SessionError::Expired | SessionError::Revoked | SessionError::NotFound => {
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
    fn status_code_session_no_autorizado() {
        // Errores de sesión que son "culpa del cliente" -> 401
        assert_eq!(WsError::Session(SessionError::Expired).status_code(), 401);
        assert_eq!(WsError::Session(SessionError::Revoked).status_code(), 401);
        assert_eq!(WsError::Session(SessionError::NotFound).status_code(), 401);
    }

    #[test]
    fn status_code_session_error_interno() {
        // El lock envenenado es un fallo del servidor -> 500
        assert_eq!(
            WsError::Session(SessionError::LockEnvenenado).status_code(),
            500
        );
    }

    #[test]
    fn clode_code_autenticacion_devuelve_1008() {
        // status 401 -> close code 1008 (policy violation)
        assert_eq!(WsError::TokenAusente.clode_code(), 1008);
        assert_eq!(WsError::TokenInvalido.clode_code(), 1008);
        assert_eq!(WsError::Session(SessionError::NotFound).clode_code(), 1008);
    }

    #[test]
    fn clode_code_error_interno_devuelve_1011() {
        // status 500 -> close code 1011 (internal error)
        assert_eq!(
            WsError::Session(SessionError::LockEnvenenado).clode_code(),
            1011
        );
    }

    #[test]
    fn client_message_por_codigo() {
        // el codigo 401 devuelve el mensaje de "no autorizado"
        assert_eq!(WsError::client_message(401), "no autorizado");

        // el codigo 500 devuelve el mensaje generico
        assert_eq!(WsError::client_message(500), "error interno");

        // cualquier otro codigo tambien cae en el mensaje generico
        assert_eq!(WsError::client_message(418), "error interno");
    }

    #[test]
    fn session_status_code_return() {
        // errores de sesion que son culpa del cliente -> 401
        assert_eq!(WsError::Session(SessionError::Expired).status_code(), 401);
        assert_eq!(WsError::Session(SessionError::Revoked).status_code(), 401);
        assert_eq!(WsError::Session(SessionError::NotFound).status_code(), 401);

        // fallo interno del servidor -> 500
        assert_eq!(WsError::Session(SessionError::LockEnvenenado).status_code(), 500);
    }

    #[test]
    fn into_ws_response_construye_respuesta() {
        // caso 401: propaga el id, el status y un mensaje seguro
        let resp = WsError::TokenAusente.into_ws_response("req-123");
        assert_eq!(resp.id, "req-123");
        assert_eq!(resp.status, 401);
        assert_eq!(resp.message, "no autorizado");
        assert_eq!(resp.data, serde_json::Value::Null);

        // caso 500: no debe filtrar el mensaje interno del error
        let resp = WsError::Session(SessionError::LockEnvenenado).into_ws_response("req-9");
        assert_eq!(resp.id, "req-9");
        assert_eq!(resp.status, 500);
        assert_eq!(resp.message, "error interno");
    }

    #[test]
    fn ws_error_into_response_http_status() {
        assert_eq!(
            WsError::TokenInvalido.into_response().status(),
            StatusCode::UNAUTHORIZED
        );
        assert_eq!(
            WsError::Session(SessionError::LockEnvenenado)
                .into_response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn api_error_into_response_http_status() {
        // credenciales invalidas -> 401
        assert_eq!(
            ApiError::CredencialesInvalidas.into_response().status(),
            StatusCode::UNAUTHORIZED
        );

        // error de sesion -> 500 (error interno)
        assert_eq!(
            ApiError::Session(SessionError::NotFound)
                .into_response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );

        // error de servicio (base de datos) -> 500
        assert_eq!(
            ApiError::Service(ServiceError::Database(sqlx::Error::RowNotFound))
                .into_response()
                .status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
