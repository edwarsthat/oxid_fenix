use crate::{
    models::{
        administracion::personal::{
            PersonalAddPayload, PersonalReadPayload, PersonalUpdatePayload,
        },
        validations::Validar,
    },
    routes::protocol::{Ctx, WsResponse},
    services::{
        administracion::personal::{add_personal, get_personal, update_personal},
        logs::audit_logs::create_audit_log,
    },
};

pub async fn personal_read(ctx: Ctx) -> WsResponse {
    let payload: PersonalReadPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let filtros = match payload.validar() {
        Ok(filtros) => filtros,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    match get_personal(&ctx.state.pool, filtros).await {
        Ok(personal) => WsResponse::ok(ctx.id, serde_json::json!({ "data": personal})),
        Err(err) => WsResponse::from_service_error(ctx.id, "personal_read", err),
    }
}

pub async fn personal_add(ctx: Ctx) -> WsResponse {
    let payload: PersonalAddPayload =
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
        Err(err) => return WsResponse::internal_error(ctx.id, "personal_add", err),
    };

    let empleado_nuevo = match add_personal(&mut *tx, datos).await {
        Ok(empleado) => empleado,
        Err(err) => return WsResponse::from_service_error(ctx.id, "personal_add", err),
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "personal",
        empleado_nuevo.id,
        "add",
        ctx.user_id,
        Some("administracion"),
        Some(serde_json::json!({
            "codigo": empleado_nuevo.codigo,
            "tipo_documento": empleado_nuevo.tipo_documento,
            "documento": empleado_nuevo.documento,
            "nombre": empleado_nuevo.nombre,
            "apellido": empleado_nuevo.apellido,
            "cargo_id": empleado_nuevo.cargo_id,
            "fecha_ingreso": empleado_nuevo.fecha_ingreso,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "personal_add", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "personal_add", err);
    }

    ctx.emit(
        "personal",
        "add",
        serde_json::json!({ "data": empleado_nuevo }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": empleado_nuevo }))
}

pub async fn personal_update(ctx: Ctx) -> WsResponse {
    let payload: PersonalUpdatePayload =
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
        Err(err) => return WsResponse::internal_error(ctx.id, "personal_update", err),
    };

    let empleado_actualizado = match update_personal(&mut *tx, datos).await {
        Ok(empleado) => empleado,
        Err(err) => return WsResponse::from_service_error(ctx.id, "personal_update", err),
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "personal",
        empleado_actualizado.id,
        "update",
        ctx.user_id,
        Some("administracion"),
        Some(serde_json::json!({
            "codigo": empleado_actualizado.codigo,
            "tipo_documento": empleado_actualizado.tipo_documento,
            "documento": empleado_actualizado.documento,
            "nombre": empleado_actualizado.nombre,
            "apellido": empleado_actualizado.apellido,
            "cargo_id": empleado_actualizado.cargo_id,
            "fecha_ingreso": empleado_actualizado.fecha_ingreso,
            "version": empleado_actualizado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "personal_update", err);
    };

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "personal_update", err);
    }

    ctx.emit(
        "personal",
        "update",
        serde_json::json!({ "data": empleado_actualizado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": empleado_actualizado }))
}
