use crate::controller::administracion as controller;
use crate::routes::protocol::{Ctx, WsResponse};

/// Router del dominio `auth` (dentro del área `sistema`).
/// resto: "usuario:listar"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    match resto {
        //Cargos
        "cargos:read" => {
            if !ctx.permisos.contains("cargos:read") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargo::cargos_read(ctx).await
        }
        "cargos:permisos:read" => {
            if !ctx.permisos.contains("cargos:read") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargo::cargo_permisos_read(ctx).await
        }
        "cargos:add" => {
            if !ctx.permisos.contains("cargos:add") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargo::cargos_add(ctx).await
        }
        "cargos:update" => {
            if !ctx.permisos.contains("cargos:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargo::cargos_update(ctx).await
        }
        "cargos:delete" => {
            if !ctx.permisos.contains("cargos:delete") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargo::cargos_delete(ctx).await
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
