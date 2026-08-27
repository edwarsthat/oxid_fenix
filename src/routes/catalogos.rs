use crate::controller::catalogos as controller;
use crate::routes::protocol::{Ctx, WsResponse};

pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    match resto {
        "materias_primas:read" => {
            if !ctx.permisos.contains("materias_primas:read") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::materias_primas::materias_primas_get(ctx).await
        }
        _ => WsResponse::error(ctx.id, 404, "Acción desconocida"),
    }
}
