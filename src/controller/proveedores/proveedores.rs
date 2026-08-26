use crate::{
    models::{
        proveedores::proveedores::{
            ProveedorAddPayload, ProveedorIdPayload, ProveedorReadPayload, ProveedorUpdatePayload,
        },
        validations::Validar,
    },
    routes::protocol::{Ctx, WsResponse},
    services::{
        logs::audit_logs::create_audit_log,
        proveedores::proveedores::{
            activar_proveedor, add_proveedor, delete_proveedor, get_proveedores, update_proveedor,
        },
    },
};

pub async fn proveedores_add(ctx: Ctx) -> WsResponse {
    let payload: ProveedorAddPayload =
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
        Err(err) => return WsResponse::internal_error(ctx.id, "proveedores_add", err),
    };

    let proveedor_nuevo = match add_proveedor(&mut *tx, datos).await {
        Ok(proveedor) => proveedor,
        Err(err) => return WsResponse::from_service_error(ctx.id, "proveedores_add", err),
    };

    // El log va dentro de la misma transacción que el INSERT: si el commit
    // falla, no queda un registro de un alta que nunca ocurrió.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "proveedores",
        proveedor_nuevo.id,
        "add",
        ctx.user_id,
        Some("proveedores"),
        Some(serde_json::json!({
            "codigo": proveedor_nuevo.codigo,
            "tipo_proveedor": proveedor_nuevo.tipo_proveedor,
            "tipo_persona": proveedor_nuevo.tipo_persona,
            "tipo_documento": proveedor_nuevo.tipo_documento,
            "documento": proveedor_nuevo.documento,
            "nombre": proveedor_nuevo.nombre,
            "razon_social": proveedor_nuevo.razon_social,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "proveedores_add", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "proveedores_add", err);
    }

    ctx.emit(
        "proveedores",
        "add",
        serde_json::json!({ "data": proveedor_nuevo }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": proveedor_nuevo }))
}

pub async fn proveedores_get(ctx: Ctx) -> WsResponse {
    let payload: ProveedorReadPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let filtros = match payload.validar() {
        Ok(filtros) => filtros,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    match get_proveedores(&ctx.state.pool, filtros).await {
        Ok(proveedores) => WsResponse::ok(ctx.id, serde_json::json!({ "data": proveedores })),
        Err(err) => WsResponse::from_service_error(ctx.id, "proveedores_read", err),
    }
}

pub async fn proveedores_update(ctx: Ctx) -> WsResponse {
    let payload: ProveedorUpdatePayload =
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
        Err(err) => return WsResponse::internal_error(ctx.id, "proveedores_update", err),
    };

    let proveedor_actualizado = match update_proveedor(&mut tx, datos).await {
        Ok(proveedor) => proveedor,
        Err(err) => return WsResponse::from_service_error(ctx.id, "proveedores_update", err),
    };

    // El log va dentro de la misma transacción que el UPDATE: si el commit
    // falla, no queda un registro de una edición que nunca ocurrió.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "proveedores",
        proveedor_actualizado.id,
        "update",
        ctx.user_id,
        Some("proveedores"),
        Some(serde_json::json!({
            "codigo": proveedor_actualizado.codigo,
            "tipo_proveedor": proveedor_actualizado.tipo_proveedor,
            "tipo_persona": proveedor_actualizado.tipo_persona,
            "tipo_documento": proveedor_actualizado.tipo_documento,
            "documento": proveedor_actualizado.documento,
            "nombre": proveedor_actualizado.nombre,
            "razon_social": proveedor_actualizado.razon_social,
            "version": proveedor_actualizado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "proveedores_update", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "proveedores_update", err);
    }

    ctx.emit(
        "proveedores",
        "update",
        serde_json::json!({ "data": proveedor_actualizado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": proveedor_actualizado }))
}

pub async fn proveedores_delete(ctx: Ctx) -> WsResponse {
    let payload: ProveedorIdPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let proveedor_id = match payload.validar() {
        Ok(id) => id,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "proveedores_delete", err),
    };

    let proveedor_retirado = match delete_proveedor(&mut *tx, proveedor_id).await {
        Ok(proveedor) => proveedor,
        Err(err) => return WsResponse::from_service_error(ctx.id, "proveedores_delete", err),
    };

    // El log va dentro de la misma transacción que la baja: si el commit falla,
    // no queda un registro de algo que nunca ocurrió.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "proveedores",
        proveedor_retirado.id,
        "delete",
        ctx.user_id,
        Some("proveedores"),
        Some(serde_json::json!({
            "codigo": proveedor_retirado.codigo,
            "tipo_proveedor": proveedor_retirado.tipo_proveedor,
            "tipo_documento": proveedor_retirado.tipo_documento,
            "documento": proveedor_retirado.documento,
            "nombre": proveedor_retirado.nombre,
            "razon_social": proveedor_retirado.razon_social,
            "version": proveedor_retirado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "proveedores_delete", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "proveedores_delete", err);
    }

    ctx.emit(
        "proveedores",
        "delete",
        serde_json::json!({ "data": proveedor_retirado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": proveedor_retirado }))
}

pub async fn proveedores_activar(ctx: Ctx) -> WsResponse {
    let payload: ProveedorIdPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let proveedor_id = match payload.validar() {
        Ok(id) => id,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "proveedores_activar", err),
    };

    let proveedor_activado = match activar_proveedor(&mut *tx, proveedor_id).await {
        Ok(proveedor) => proveedor,
        Err(err) => return WsResponse::from_service_error(ctx.id, "proveedores_activar", err),
    };

    // El log va dentro de la misma transacción que la reactivación: si el commit
    // falla, no queda un registro de algo que nunca ocurrió.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "proveedores",
        proveedor_activado.id,
        "activar",
        ctx.user_id,
        Some("proveedores"),
        Some(serde_json::json!({
            "codigo": proveedor_activado.codigo,
            "tipo_proveedor": proveedor_activado.tipo_proveedor,
            "tipo_documento": proveedor_activado.tipo_documento,
            "documento": proveedor_activado.documento,
            "nombre": proveedor_activado.nombre,
            "razon_social": proveedor_activado.razon_social,
            "activo": proveedor_activado.activo,
            "version": proveedor_activado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "proveedores_activar", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "proveedores_activar", err);
    }

    ctx.emit(
        "proveedores",
        "update",
        serde_json::json!({ "data": proveedor_activado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": proveedor_activado }))
}
