use std::{collections::HashSet, sync::Arc};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{app::app::AppState, services::error::ServiceError};

#[derive(Deserialize, Debug)]
pub struct WsRequest {
    pub id: String,
    pub token: String,
    pub payload: WsPayload,
}

#[derive(Deserialize, Debug)]
pub struct WsPayload {
    pub action: String,
    #[serde(flatten)]
    pub data: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Debug)]
pub struct WsResponse {
    pub id: String,
    pub status: i32,
    pub message: String,
    #[serde(flatten)]
    pub data: serde_json::Value,
}

#[derive(Serialize)]
pub struct WsEvent {
    pub event: String,
    pub action: String,
    pub data: serde_json::Value,
}

pub struct Ctx {
    pub state: AppState,
    pub id: String,
    pub user_id: Uuid,
    pub data: serde_json::Map<String, serde_json::Value>,
    pub token: String,
    pub permisos: Arc<HashSet<String>>,
}

impl Ctx {
    pub fn emit(&self, event: &str, action: &str, data: serde_json::Value) {
        let evento = WsEvent {
            event: event.into(),
            action: action.into(),
            data,
        };
        if let Ok(json) = serde_json::to_string(&evento) {
            let _ = self.state.eventos.send(json);
        }
    }
}

impl WsResponse {
    pub fn ok(id: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            id: id.into(),
            status: 200,
            message: "Ok".to_string(),
            data,
        }
    }
    pub fn error(id: impl Into<String>, status: i32, msg: &str) -> Self {
        Self {
            id: id.into(),
            status,
            message: msg.to_string(),
            data: serde_json::Value::Null,
        }
    }

    pub fn internal_error(id: impl Into<String>, ctx: &str, err: impl std::fmt::Display) -> Self {
        tracing::error!("[{ctx}] {err}");
        Self::error(id, 500, "error interno")
    }

    pub fn from_service_error(id: impl Into<String>, ctx: &str, err: ServiceError) -> Self {
        match err {
            ServiceError::NotFound(msg) => Self::error(id, 404, &msg),
            ServiceError::Conflict(msg) => Self::error(id, 409, &msg),
            ServiceError::BadRequest(msg) => Self::error(id, 400, &msg),
            err => Self::internal_error(id, ctx, err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_service_error_not_found_devuelve_404() {
        let resp = WsResponse::from_service_error(
            "id-1",
            "ctx",
            ServiceError::NotFound("cargo no encontrado".into()),
        );

        assert_eq!(resp.id, "id-1");
        assert_eq!(resp.status, 404);
        assert_eq!(resp.message, "cargo no encontrado");
    }

    #[test]
    fn from_service_error_conflict_devuelve_409() {
        let resp = WsResponse::from_service_error(
            "id-2",
            "ctx",
            ServiceError::Conflict("ya existe un cargo con ese nombre".into()),
        );

        assert_eq!(resp.status, 409);
        assert_eq!(resp.message, "ya existe un cargo con ese nombre");
    }

    #[test]
    fn from_service_error_bad_request_devuelve_400() {
        let resp = WsResponse::from_service_error(
            "id-3",
            "ctx",
            ServiceError::BadRequest("permiso invalido".into()),
        );

        assert_eq!(resp.status, 400);
        assert_eq!(resp.message, "permiso invalido");
    }

    #[test]
    fn from_service_error_database_cae_en_error_interno() {
        // un error de base de datos "crudo" no debe filtrar detalles al cliente
        let resp = WsResponse::from_service_error(
            "id-4",
            "ctx",
            ServiceError::Database(sqlx::Error::RowNotFound),
        );

        assert_eq!(resp.status, 500);
        assert_eq!(resp.message, "error interno");
    }
}
