use crate::{
    models::{
        inventarios::llaves_nfc::{LlaveNfcAddPayload, LlaveNfcReadPayload, LlaveNfcUpdatePayload},
        validations::Validar,
    },
    routes::protocol::{Ctx, WsResponse},
    services::{
        inventarios::llaves_nfc::{add_llave_nfc, get_llaves_nfc, update_llave_nfc},
        logs::audit_logs::create_audit_log,
    },
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

    // El evento tiene que llamarse igual que el permiso: `emit` arma el filtro
    // como `{event}:read` y el permiso sembrado es `llaves_nfc:read`.
    ctx.emit(
        "llaves_nfc",
        "add",
        serde_json::json!({ "data": llave_nfc_nueva }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": llave_nfc_nueva }))
}

pub async fn llave_nfc_read(ctx: Ctx) -> WsResponse {
    let payload: LlaveNfcReadPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let filtros = match payload.validar() {
        Ok(filtros) => filtros,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    match get_llaves_nfc(&ctx.state.pool, filtros).await {
        Ok(llave_nfc) => WsResponse::ok(ctx.id, serde_json::json!({ "data": llave_nfc })),
        Err(err) => WsResponse::from_service_error(ctx.id, "llave_nfc_read", err),
    }
}

pub async fn llave_nfc_update(ctx: Ctx) -> WsResponse {
    let payload: LlaveNfcUpdatePayload =
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
        Err(err) => return WsResponse::internal_error(ctx.id, "llave_nfc_update", err),
    };

    let llave_nfc_actualizado = match update_llave_nfc(&mut tx, datos).await {
        Ok(llave_nfc) => llave_nfc,
        Err(err) => return WsResponse::from_service_error(ctx.id, "llave_nfc_update", err),
    };

    // Se registra el estado como quedó, no el que mandó el cliente: si el
    // trigger o un DEFAULT tocaron algo, el log tiene que reflejar la fila real.
    // uid y codigo van aunque no sean editables: identifican la tarjeta física
    // en el log sin tener que salir a buscarla por id.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "llaves_nfc",
        llave_nfc_actualizado.id,
        "update",
        ctx.user_id,
        Some("inventarios"),
        Some(serde_json::json!({
            "uid": llave_nfc_actualizado.uid,
            "codigo": llave_nfc_actualizado.codigo,
            "estado": llave_nfc_actualizado.estado,
            "descripcion": llave_nfc_actualizado.descripcion,
            "version": llave_nfc_actualizado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "llave_nfc_update", err);
    };

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "llave_nfc_update", err);
    }

    ctx.emit(
        "llaves_nfc",
        "update",
        serde_json::json!({ "data": llave_nfc_actualizado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": llave_nfc_actualizado }))
}
