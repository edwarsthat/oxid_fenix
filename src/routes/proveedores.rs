use crate::controller::proveedores as controller;
use crate::routes::protocol::{Ctx, WsResponse};

pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    match resto {
        "proveedores:add" => {
            if !ctx.permisos.contains("proveedores:add") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::proveedores::proveedores_add(ctx).await
        }
        "proveedores:read" => {
            if !ctx.permisos.contains("proveedores:read") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::proveedores::proveedores_get(ctx).await
        }
        "proveedores:update" => {
            if !ctx.permisos.contains("proveedores:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::proveedores::proveedores_update(ctx).await
        }
        "proveedores:delete" => {
            if !ctx.permisos.contains("proveedores:delete") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::proveedores::proveedores_delete(ctx).await
        }
        "proveedores:reactivar" => {
            if !ctx.permisos.contains("proveedores:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::proveedores::proveedores_activar(ctx).await
        }
        "predios:add" => {
            if !ctx.permisos.contains("predios:add") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::predios::predios_add(ctx).await
        }
        "predios:read" => {
            if !ctx.permisos.contains("predios:read") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::predios::predios_get(ctx).await
        }
        "predios:update" => {
            if !ctx.permisos.contains("predios:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::predios::predios_update(ctx).await
        }
        "predios:delete" => {
            if !ctx.permisos.contains("predios:delete") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::predios::predio_delete(ctx).await
        }
        "predios:reactivar" => {
            if !ctx.permisos.contains("predios:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::predios::predio_activar(ctx).await
        }
        _ => WsResponse::error(ctx.id, 404, "Acción desconocida"),
    }
}
