use crate::{
    routes::protocol::{Ctx, WsResponse},
    services::administracion::permisos::get_permisos,
};

pub async fn permisos_read(ctx: Ctx) -> WsResponse {
    match get_permisos(&ctx.state.pool).await {
        Ok(permisos) => WsResponse::ok(ctx.id, serde_json::json!({ "data": permisos })),
        Err(err) => WsResponse::internal_error(ctx.id, "permisos_read", err),
    }
}
