use crate::{
    models::{
        proveedores::proveedores::{ProveedorAddPayload, ProveedorReadPayload},
        validations::Validar,
    },
    routes::protocol::{Ctx, WsResponse},
    services::{
        logs::audit_logs::create_audit_log,
        proveedores::proveedores::{add_proveedor, get_proveedores},
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
