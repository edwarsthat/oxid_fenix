
use crate::{
    routes::protocol::{Ctx, WsResponse}, 
    services::administracion::cargos::get_cargos
};


pub async fn cargos_read(ctx: Ctx) -> WsResponse {
    match get_cargos(&ctx.state.pool).await {
        Ok(cargos) => WsResponse::ok(ctx.id, serde_json::json!({ "data": cargos })),
        Err(err) => {
            eprintln!("[cargos_read] {err}");
            WsResponse::error(ctx.id, 500, "error interno")
        }
    }
}