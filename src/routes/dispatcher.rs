use serde::{Deserialize, Serialize};
use serde_json;


#[derive(Deserialize)]
pub struct WsRequest {
    pub action: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Serialize)]
pub struct WsResponse {
    pub status: i32,
    pub message: String, 
    pub data: serde_json::Value,
}

impl WsResponse {
    fn ok(data: serde_json::Value) -> Self {
        Self { status: 200, message: "Ok".to_string(), data }
    }
    fn error(status: i32, msg: &str) -> Self {
        Self {
            status,
            message: msg.to_string(),
            data: serde_json::Value::Null,
        }
    }
}