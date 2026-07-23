use crate::{
    routes::protocol::{Ctx, WsResponse},
    services::administracion::usuarios::get_usuarios,
};

pub async fn usuarios_read(ctx: Ctx) -> WsResponse {
    match get_usuarios(&ctx.state.pool).await {
        Ok(usuarios) => WsResponse::ok(ctx.id, serde_json::json!({ "data": usuarios })),
        Err(err) => WsResponse::from_service_error(ctx.id, "usuarios_read", err),
    }
}
