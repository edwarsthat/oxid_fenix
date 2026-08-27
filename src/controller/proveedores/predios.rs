use crate::{
    models::{proveedores::predios::PredioAddPayload, validations::Validar},
    routes::protocol::{Ctx, WsResponse},
    services::{logs::audit_logs::create_audit_log, proveedores::predios::add_predios},
};

pub async fn predios_add(ctx: Ctx) -> WsResponse {
    let payload: PredioAddPayload =
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
        Err(err) => return WsResponse::internal_error(ctx.id, "predios_add", err),
    };

    let predio_nuevo = match add_predios(&mut *tx, datos).await {
        Ok(predio) => predio,
        Err(err) => return WsResponse::from_service_error(ctx.id, "predios_add", err),
    };

    // El log va dentro de la misma transacción que el INSERT: si el commit
    // falla, no queda un registro de un alta que nunca ocurrió.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "predios",
        predio_nuevo.id,
        "add",
        ctx.user_id,
        Some("proveedores"),
        Some(serde_json::json!({
            "codigo": predio_nuevo.codigo,
            "proveedor_id": predio_nuevo.proveedor_id,
            "nombre": predio_nuevo.nombre,
            "departamento": predio_nuevo.departamento,
            "municipio": predio_nuevo.municipio,
            "vereda": predio_nuevo.vereda,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "predios_add", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "predios_add", err);
    }

    ctx.emit("predios", "add", serde_json::json!({ "data": predio_nuevo }));

    WsResponse::ok(ctx.id, serde_json::json!({ "data": predio_nuevo }))
}
