use crate::{
    routes::protocol::{Ctx, WsResponse},
    services::administracion::permisos::get_permisos,
};

pub async fn permisos_read(ctx: Ctx) -> WsResponse {
    match get_permisos(&ctx.state.pool).await {
        Ok(permisos) => WsResponse::ok(ctx.id, serde_json::json!({ "data": permisos })),
        Err(err) => {
            eprintln!("[cargos_read] {err}");
            WsResponse::error(ctx.id, 500, "error interno")
        }
    }
}
