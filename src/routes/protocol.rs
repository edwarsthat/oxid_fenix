use serde::{Deserialize, Serialize};

use crate::app::app::AppState;

#[derive(Deserialize, Debug)]
pub struct WsRequest {
    pub id: String,
    pub action: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Serialize, Debug)]
pub struct WsResponse {
    pub id: String,
    pub status: i32,
    pub message: String,
    pub data: serde_json::Value,
}

#[derive(Serialize)]
pub struct WsEvent {
    pub event: String,
    pub data: serde_json::Value,
}

pub struct Ctx {
    pub state: AppState,
    pub id: String,
    pub payload: serde_json::Value,
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
}

