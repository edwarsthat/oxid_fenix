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
        "usuarios:read" => {
            if !ctx.permisos.contains("usuarios:read") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::usuarios::usuarios_read(ctx).await
        }
        "usuarios:add" => {
            if !ctx.permisos.contains("usuarios:add") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::usuarios::usuarios_add(ctx).await
        }
        "usuarios:update" => {
            if !ctx.permisos.contains("usuarios:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::usuarios::usuarios_update(ctx).await
        }
        "usuarios:delete" => {
            if !ctx.permisos.contains("usuarios:delete") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::usuarios::usuarios_delete(ctx).await
        }
        "usuarios:newpassword" => {
            if !ctx.permisos.contains("usuarios:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::usuarios::usuarios_new_password(ctx).await
        }
        "usuarios:reactivar" => {
            if !ctx.permisos.contains("usuarios:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::usuarios::usuarios_activar(ctx).await
        }
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
        _ => WsResponse::error(ctx.id, 404, "Acción desconocida"),
    }
}
