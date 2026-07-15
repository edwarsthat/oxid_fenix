use crate::controller::administracion as controller;
use crate::routes::protocol::{Ctx, WsResponse};

/// Router del dominio `auth` (dentro del área `sistema`).
/// resto: "usuario:listar"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    match resto {
        //Cargos
        "cargos:read" => {
            if !ctx.permisos.contains(resto) {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargo::cargos_read(ctx).await
        }
        "cargos:create" => {
            if !ctx.permisos.contains(resto) {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargo::cargos_read(ctx).await
        }
        "permisos:read" => {
            if !ctx.permisos.contains(resto) {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::permisos::permisos_read(ctx).await
        }

        _ => WsResponse::error(ctx.id, 404, "Acción desconocida"),
    }
}
