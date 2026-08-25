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
        _ => WsResponse::error(ctx.id, 404, "Acción desconocida"),
    }
}
