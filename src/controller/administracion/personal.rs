use uuid::Uuid;

use crate::{
    models::{
        administracion::personal::{
            PersonalAddPayload, PersonalReadPayload, PersonalUpdatePayload,
        },
        validations::Validar,
    },
    routes::protocol::{Ctx, WsResponse},
    services::{
        administracion::personal::{
            activar_personal, add_personal, delete_personal, get_personal, update_personal,
        },
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

pub async fn personal_delete(ctx: Ctx) -> WsResponse {
    let empleado_id = match ctx.data.get("empleado_id").and_then(|v| v.as_str()) {
        Some(empleado_id) => empleado_id,
        None => return WsResponse::error(ctx.id, 400, "Falta el empleado id"),
    };

    let empleado_id = match Uuid::parse_str(empleado_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "El empleado_id no es un UUID válido"),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "personal_delete", err),
    };

    let empleado_retirado = match delete_personal(&mut *tx, empleado_id).await {
        Ok(empleado) => empleado,
        Err(err) => return WsResponse::from_service_error(ctx.id, "personal_delete", err),
    };

    // fecha_retiro la pone el servidor con CURRENT_DATE, así que si no queda
    // registrada acá no hay contra qué contrastarla si alguien la edita después.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "personal",
        empleado_retirado.id,
        "delete",
        ctx.user_id,
        Some("administracion"),
        Some(serde_json::json!({
            "codigo": empleado_retirado.codigo,
            "activo": empleado_retirado.activo,
            "fecha_retiro": empleado_retirado.fecha_retiro,
            "version": empleado_retirado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "personal_delete", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "personal_delete", err);
    }

    ctx.emit(
        "personal",
        "delete",
        serde_json::json!({ "data": empleado_retirado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": empleado_retirado }))
}

pub async fn personal_activar(ctx: Ctx) -> WsResponse {
    let empleado_id = match ctx.data.get("empleado_id").and_then(|v| v.as_str()) {
        Some(empleado_id) => empleado_id,
        None => return WsResponse::error(ctx.id, 400, "Falta el empleado id"),
    };

    let empleado_id = match Uuid::parse_str(empleado_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "El empleado_id no es un UUID válido"),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "personal_activar", err),
    };

    let empleado_activado = match activar_personal(&mut *tx, empleado_id).await {
        Ok(empleado) => empleado,
        Err(err) => return WsResponse::from_service_error(ctx.id, "personal_activar", err),
    };

    // La nueva fecha_ingreso pisa la del contrato anterior, así que este registro
    // es el único rastro de cuándo se hizo el reingreso y con qué fecha quedó.
    if let Err(err) = create_audit_log(
        &mut *tx,
        "personal",
        empleado_activado.id,
        "activar",
        ctx.user_id,
        Some("administracion"),
        Some(serde_json::json!({
            "codigo": empleado_activado.codigo,
            "activo": empleado_activado.activo,
            "fecha_ingreso": empleado_activado.fecha_ingreso,
            "version": empleado_activado.version,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "personal_activar", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "personal_activar", err);
    }

    ctx.emit(
        "personal",
        "update",
        serde_json::json!({ "data": empleado_activado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": empleado_activado }))
}
