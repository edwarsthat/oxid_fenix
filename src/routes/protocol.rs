use std::{collections::HashSet, sync::Arc};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::app::app::AppState;

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
    pub data: serde_json::Map<String, serde_json::Value>
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
    pub data: serde_json::Value,
    pub token: String,
}

pub struct Ctx {
    pub state: AppState,
    pub id: String,
    pub user_id: Uuid,
    pub data:  serde_json::Map<String, serde_json::Value>,
    pub token: String,
    pub permisos: Arc<HashSet<String>>

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
}

