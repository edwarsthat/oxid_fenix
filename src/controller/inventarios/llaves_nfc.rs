use crate::{
    models::{inventarios::llaves_nfc::LlaveNfcAddPayload, validations::Validar},
    routes::protocol::{Ctx, WsResponse},
    services::{inventarios::llaves_nfc::add_llave_nfc, logs::audit_logs::create_audit_log},
};

pub async fn llaves_nfc_add(ctx: Ctx) -> WsResponse {
    let payload: LlaveNfcAddPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let datos = match payload.validar() {
        Ok(datos) => datos,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "llaves_nfc_add", err),
    };

    let llave_nfc_nueva = match add_llave_nfc(&mut *tx, datos).await {
        Ok(llave_nfc) => llave_nfc,
        Err(err) => return WsResponse::from_service_error(ctx.id, "llaves_nfc_add", err),
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "llaves_nfc",
        llave_nfc_nueva.id,
        "add",
        ctx.user_id,
        Some("inventarios"),
        Some(serde_json::json!({
            "uid": llave_nfc_nueva.uid,
            "codigo": llave_nfc_nueva.codigo,
            "descripcion": llave_nfc_nueva.descripcion,
            "creado_en": llave_nfc_nueva.creado_en,
            "estado": llave_nfc_nueva.estado
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "llaves_nfc_add", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "llaves_nfc_add", err);
    }

    ctx.emit(
        "llave_nfc",
        "add",
        serde_json::json!({ "data": llave_nfc_nueva }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": llave_nfc_nueva }))
}
