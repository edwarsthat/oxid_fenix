use crate::controller::inventarios as controller;
use crate::routes::protocol::{Ctx, WsResponse};

/// Router del área `inventarios`: las cosas que la empresa compra y guarda.
/// La llave existe acá como objeto físico; a quién se le entrega es asunto de
/// `talento_humano`.
/// resto: "llaves_nfc:add"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    match resto {
        //Llaves NFC
        "llaves_nfc:add" => {
            if !ctx.permisos.contains("llaves_nfc:add") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::llaves_nfc::llaves_nfc_add(ctx).await
        }
        "llaves_nfc:read" => {
            if !ctx.permisos.contains("llaves_nfc:read") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::llaves_nfc::llave_nfc_read(ctx).await
        }
        "llaves_nfc:update" => {
            if !ctx.permisos.contains("llaves_nfc:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::llaves_nfc::llave_nfc_update(ctx).await
        }
        "llaves_nfc:asignar_llave" => {
            if !ctx.permisos.contains("llaves_nfc:update") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::llaves_nfc::controller_asignar_llave(ctx).await
        }
        _ => WsResponse::error(ctx.id, 404, "Acción desconocida"),
    }
}
