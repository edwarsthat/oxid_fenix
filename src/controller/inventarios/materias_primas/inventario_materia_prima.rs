use crate::{
    routes::protocol::{Ctx, WsResponse},
    services::inventarios::materias_primas::inventario_materia_prima::read_inventario_materia_prima,
};

pub async fn inventario_materia_prima_read(ctx: Ctx) -> WsResponse {
    match read_inventario_materia_prima(&ctx.state.pool).await {
        Ok(lotes) => WsResponse::ok(ctx.id, serde_json::json!({ "data": lotes })),
        Err(err) => WsResponse::from_service_error(ctx.id, "inventario_materia_prima_read", err),
    }
}
