use crate::{
    models::{
        talento_humano::cargos_personal::{
            CargoPersonalAddPayload, CargoPersonalIdPayload, CargoPersonalUpdatePayload,
        },
        validations::Validar,
    },
    routes::protocol::{Ctx, WsResponse},
    services::{
        logs::audit_logs::create_audit_log,
        talento_humano::cargos_personal::{
            activar_cargo_personal, create_cargo_personal, delete_cargo_personal,
            get_cargo_personal, update_cargo_personal,
        },
    },
};

pub async fn cargos_personal_add(ctx: Ctx) -> WsResponse {
    let payload: CargoPersonalAddPayload =
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
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_personal_add", err),
    };

    let nuevo_cargo_personal =
        match create_cargo_personal(&mut *tx, &datos.nombre, &datos.tipo_contrato).await {
            Ok(cargo) => cargo,
            Err(err) => return WsResponse::from_service_error(ctx.id, "cargos_personal_add", err),
        };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "cargos_personal",
        nuevo_cargo_personal.id,
        "add",
        ctx.user_id,
        Some("administracion"),
        Some(serde_json::json!({
            "nombre": nuevo_cargo_personal.nombre,
            "tipo_contrato": nuevo_cargo_personal.tipo_contrato
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "cargos_personal_add", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "cargos_personal_add", err);
    }

    ctx.emit(
        "cargos_personal",
        "add",
        serde_json::json!({ "data": nuevo_cargo_personal }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": nuevo_cargo_personal }))
}

pub async fn cargos_personal_update(ctx: Ctx) -> WsResponse {
    let payload: CargoPersonalUpdatePayload =
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
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_personal_update", err),
    };

    let cargo_personal_actualizado = match update_cargo_personal(
        &mut *tx,
        &datos.nombre,
        &datos.tipo_contrato,
        datos.id,
    )
    .await
    {
        Ok(cargo) => cargo,
        Err(err) => return WsResponse::from_service_error(ctx.id, "cargos_personal_update", err),
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "cargos_personal",
        cargo_personal_actualizado.id,
        "update",
        ctx.user_id,
        Some("administracion"),
        Some(serde_json::json!({ "cargo_personal": cargo_personal_actualizado })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "cargos_personal_update", err);
    };

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "cargos_personal_update", err);
    }

    ctx.emit(
        "cargos_personal",
        "update",
        serde_json::json!({ "data": cargo_personal_actualizado }),
    );

    WsResponse::ok(
        ctx.id,
        serde_json::json!({ "data": cargo_personal_actualizado }),
    )
}

pub async fn cargos_personal_delete(ctx: Ctx) -> WsResponse {
    let payload: CargoPersonalIdPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let cargo_id = match payload.validar() {
        Ok(id) => id,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_personal_delete", err),
    };

    if let Err(err) = delete_cargo_personal(&mut *tx, cargo_id).await {
        return WsResponse::from_service_error(ctx.id, "cargos_personal_delete", err);
    }

    if let Err(err) = create_audit_log(
        &mut *tx,
        "cargos_personal",
        cargo_id,
        "delete",
        ctx.user_id,
        Some("administracion"),
        None,
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "cargos_personal_delete", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "cargos_personal_delete", err);
    }

    ctx.emit(
        "cargos_personal",
        "delete",
        serde_json::json!({ "cargo_id": cargo_id }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "cargo_id": cargo_id }))
}

pub async fn cargos_personal_read(ctx: Ctx) -> WsResponse {
    match get_cargo_personal(&ctx.state.pool).await {
        Ok(cargos) => WsResponse::ok(ctx.id, serde_json::json!({ "data": cargos })),
        Err(err) => WsResponse::from_service_error(ctx.id, "cargos_personal_read", err),
    }
}

pub async fn cargos_personal_activar(ctx: Ctx) -> WsResponse {
    let payload: CargoPersonalIdPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let cargo_id = match payload.validar() {
        Ok(id) => id,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_personal_activar", err),
    };

    let cargo_activado = match activar_cargo_personal(&mut *tx, cargo_id).await {
        Ok(c) => c,
        Err(err) => return WsResponse::from_service_error(ctx.id, "cargos_personal_activar", err),
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "cargos_personal",
        cargo_activado.id,
        "activar",
        ctx.user_id,
        Some("administracion"),
        Some(serde_json::json!({ "cargo_personal": cargo_activado })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "cargos_personal_activar", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "cargos_personal_activar", err);
    }

    ctx.emit(
        "cargos_personal",
        "update",
        serde_json::json!({ "data": cargo_activado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": cargo_activado }))
}
