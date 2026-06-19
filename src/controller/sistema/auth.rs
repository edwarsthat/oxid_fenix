use crate::routes::protocol::{Ctx, WsResponse};

pub async fn list_usuarios(ctx: Ctx) -> WsResponse {
    println!("[controller::sistema::auth] Listando Usuarios (id={})", ctx.id);
    WsResponse::ok(ctx.id, serde_json::Value::Null)
}