use crate::{
    routes::protocol::{Ctx, WsResponse},
    services::catalogos::materias_primas::get_materias_primas,
};

pub async fn materias_primas_get(ctx: Ctx) -> WsResponse {
    match get_materias_primas(&ctx.state.pool).await {
        Ok(materias_primas) => {
            WsResponse::ok(ctx.id, serde_json::json!({ "data": materias_primas }))
        }
        Err(err) => WsResponse::from_service_error(ctx.id, "materias_primas_read", err),
    }
}
