use crate::controller::talento_humano as controller;
use crate::routes::protocol::{Ctx, WsResponse};

/// Router del área `talento_humano`: quién trabaja en la empresa y qué se le
/// entrega. Los cargos de acá son los del empleado, no los del sistema: esos
/// viven en `administracion`.
/// resto: "personal:read"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    match resto {
        //Cargos del personal
        "cargos_personal:read" => {
            if !ctx.permisos.contains("cargos_personal:read") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargos_personal::cargos_personal_read(ctx).await
        }
        "cargos_personal:add" => {
            if !ctx.permisos.contains("cargos_personal:add") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargos_personal::cargos_personal_add(ctx).await
        }
        "cargos_personal:update" => {
            if !ctx.permisos.contains("cargos_personal:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargos_personal::cargos_personal_update(ctx).await
        }
        "cargos_personal:delete" => {
            if !ctx.permisos.contains("cargos_personal:delete") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargos_personal::cargos_personal_delete(ctx).await
        }
        "cargos_personal:reactivar" => {
            if !ctx.permisos.contains("cargos_personal:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::cargos_personal::cargos_personal_activar(ctx).await
        }
        //Personal
        "personal:read" => {
            if !ctx.permisos.contains("personal:read") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::personal::personal_read(ctx).await
        }
        "personal:add" => {
            if !ctx.permisos.contains("personal:add") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::personal::personal_add(ctx).await
        }
        "personal:update" => {
            if !ctx.permisos.contains("personal:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::personal::personal_update(ctx).await
        }
        "personal:delete" => {
            if !ctx.permisos.contains("personal:delete") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::personal::personal_delete(ctx).await
        }
        "personal:reactivar" => {
            if !ctx.permisos.contains("personal:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::personal::personal_activar(ctx).await
        }
        _ => WsResponse::error(ctx.id, 404, "Acción desconocida"),
    }
}
