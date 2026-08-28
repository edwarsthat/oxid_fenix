use crate::{
    models::{
        proveedores::predios::{
            PredioAddPayload, PredioIdPayload, PredioUpdatePayload, PrediosReadPayload,
        },
        validations::Validar,
    },
    routes::protocol::{Ctx, WsResponse},
    services::{
        logs::audit_logs::create_audit_log,
        proveedores::predios::{
            activar_predio, add_predios, delete_predio, get_predios, update_predio,
        },
    },
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

    ctx.emit(
        "predios",
        "add",
        serde_json::json!({ "data": predio_nuevo }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": predio_nuevo }))
}

pub async fn predios_get(ctx: Ctx) -> WsResponse {
    let payload: PrediosReadPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let filtros = match payload.validar() {
        Ok(filtros) => filtros,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    match get_predios(&ctx.state.pool, filtros).await {
        Ok(predios) => WsResponse::ok(ctx.id, serde_json::json!({ "data": predios })),
        Err(err) => WsResponse::from_service_error(ctx.id, "predios_read", err),
    }
}

pub async fn predios_update(ctx: Ctx) -> WsResponse {
    let payload: PredioUpdatePayload =
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
        Err(err) => return WsResponse::internal_error(ctx.id, "predios_update", err),
    };

    let predio_actualizado = match update_predio(&mut tx, datos).await {
        Ok(predio) => predio,
        Err(err) => return WsResponse::from_service_error(ctx.id, "predios_update", err),
    };

    // El log va dentro de la misma transacción que el UPDATE: si el commit
    // falla, no queda un registro de una edición que nunca ocurrió.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "predios",
        predio_actualizado.id,
        "update",
        ctx.user_id,
        Some("proveedores"),
        Some(serde_json::json!({
            "codigo": predio_actualizado.codigo,
            "proveedor_id": predio_actualizado.proveedor_id,
            "nombre": predio_actualizado.nombre,
            "departamento": predio_actualizado.departamento,
            "municipio": predio_actualizado.municipio,
            "vereda": predio_actualizado.vereda,
            "version": predio_actualizado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "predios_update", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "predios_update", err);
    }

    ctx.emit(
        "predios",
        "update",
        serde_json::json!({ "data": predio_actualizado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": predio_actualizado }))
}

pub async fn predio_delete(ctx: Ctx) -> WsResponse {
    let payload: PredioIdPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let predio_id = match payload.validar() {
        Ok(id) => id,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "predios_delete", err),
    };

    let predio_retirado = match delete_predio(&mut *tx, predio_id).await {
        Ok(predio) => predio,
        Err(err) => return WsResponse::from_service_error(ctx.id, "predios_delete", err),
    };

    // El log va dentro de la misma transacción que la baja: si el commit falla,
    // no queda un registro de algo que nunca ocurrió.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "predios",
        predio_retirado.id,
        "delete",
        ctx.user_id,
        Some("proveedores"),
        Some(serde_json::json!({
            "codigo": predio_retirado.codigo,
            "proveedor_id": predio_retirado.proveedor_id,
            "nombre": predio_retirado.nombre,
            "departamento": predio_retirado.departamento,
            "municipio": predio_retirado.municipio,
            "vereda": predio_retirado.vereda,
            "version": predio_retirado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "predios_delete", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "predios_delete", err);
    }

    ctx.emit(
        "predios",
        "delete",
        serde_json::json!({ "data": predio_retirado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": predio_retirado }))
}
pub async fn predio_activar(ctx: Ctx) -> WsResponse {
    let payload: PredioIdPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let predio_id = match payload.validar() {
        Ok(id) => id,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "predios_activar", err),
    };

    let predio_activado = match activar_predio(&mut *tx, predio_id).await {
        Ok(predio) => predio,
        Err(err) => return WsResponse::from_service_error(ctx.id, "predios_activar", err),
    };

    // El log va dentro de la misma transacción que la reactivación: si el commit
    // falla, no queda un registro de algo que nunca ocurrió.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "predios",
        predio_activado.id,
        "activar",
        ctx.user_id,
        Some("proveedores"),
        Some(serde_json::json!({
            "codigo": predio_activado.codigo,
            "proveedor_id": predio_activado.proveedor_id,
            "nombre": predio_activado.nombre,
            "departamento": predio_activado.departamento,
            "municipio": predio_activado.municipio,
            "vereda": predio_activado.vereda,
            "activo": predio_activado.activo,
            "version": predio_activado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "predios_activar", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "predios_activar", err);
    }

    ctx.emit(
        "predios",
        "update",
        serde_json::json!({ "data": predio_activado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": predio_activado }))
}
