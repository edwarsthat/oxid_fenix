
use crate::controller::administracion::cargo as controller;
use crate::routes::protocol::{Ctx, WsResponse};

/// Router del dominio `auth` (dentro del área `sistema`).
/// resto: "usuario:listar"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    match resto {
        "cargos:read" => {
            if !ctx.permisos.contains(resto) {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargos_read().await
        }
        _ => WsResponse::error(ctx.id, 404, "Acción desconocida"),
    }
}