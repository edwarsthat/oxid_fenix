use crate::routes::protocol::{Ctx, WsResponse};



pub async fn cargos_personal_add(ctx: Ctx) -> WsResponse {


        WsResponse::ok(
        ctx.id,
        serde_json::json!({}),
    )
}