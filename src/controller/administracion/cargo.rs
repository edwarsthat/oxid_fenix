use uuid::Uuid;

use crate::{
    routes::protocol::{Ctx, WsResponse},
    services::{
        administracion::{
            cargos::{create_cargo, get_cargos, soft_delete_cargo, update_cargo},
            cargos_permisos::{add_cargo_permiso, get_permisos_de_cargo, sync_cargo_permisos},
        },
        logs::audit_logs::create_audit_log,
    },
};

pub async fn cargos_read(ctx: Ctx) -> WsResponse {
    match get_cargos(&ctx.state.pool).await {
        Ok(cargos) => WsResponse::ok(ctx.id, serde_json::json!({ "data": cargos })),
        Err(err) => WsResponse::internal_error(ctx.id, "cargos_read", err),
    }
}

pub async fn cargo_permisos_read(ctx: Ctx) -> WsResponse {
    let cargo_id = match ctx.data.get("cargo_id").and_then(|v| v.as_str()) {
        Some(cargo_id) => cargo_id,
        None => return WsResponse::error(ctx.id, 400, "Falta el cargo"),
    };

    let cargo_id: Uuid = match Uuid::parse_str(cargo_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "cargo_id no valido"),
    };

    let permisos = match get_permisos_de_cargo(&ctx.state.pool, cargo_id).await {
        Ok(permisos) => permisos,
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_read", err),
    };

    WsResponse::ok(ctx.id, serde_json::json!({ "data": permisos }))
}

pub async fn cargos_add(ctx: Ctx) -> WsResponse {
    let nombre = match ctx.data.get("nombre").and_then(|v| v.as_str()) {
        Some(nombre) if !nombre.trim().is_empty() => nombre,
        _ => return WsResponse::error(ctx.id, 400, "Error nombre no valido"),
    };

    let descripcion = match ctx.data.get("descripcion").and_then(|v| v.as_str()) {
        Some(nombre) => nombre,
        None => return WsResponse::error(ctx.id, 400, "Error descripcion no valida"),
    };

    let permisos_arr = match ctx.data.get("permisos").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr,
        _ => return WsResponse::error(ctx.id, 400, "Error permisos no validos"),
    };

    let permisos: Vec<String> = match permisos_arr
        .iter()
        .map(|v| v.as_str().map(String::from))
        .collect::<Option<Vec<String>>>()
    {
        Some(permisos) => permisos,
        None => return WsResponse::error(ctx.id, 400, "Error permisos no validos"),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_add", err),
    };

    let new_cargo = match create_cargo(&mut *tx, &nombre, &descripcion).await {
        Ok(cargo) => cargo,
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_add", err),
    };

    if let Err(err) = add_cargo_permiso(&mut *tx, new_cargo.id, permisos.clone()).await {
        return WsResponse::internal_error(ctx.id, "cargos_add", err);
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "cargo",
        new_cargo.id,
        "add",
        ctx.user_id,
        None,
        Some(serde_json::json!({ "nombre": nombre, "descripcion": descripcion, "permisos": permisos })),
    ).await {
        return WsResponse::internal_error(ctx.id, "cargos_add", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "cargos_add", err);
    }

    ctx.emit("cargos", "add", serde_json::json!({ "data": new_cargo }));
    WsResponse::ok(ctx.id, serde_json::json!({ "data": new_cargo }))
}

pub async fn cargos_update(ctx: Ctx) -> WsResponse {
    let nombre = match ctx.data.get("nombre").and_then(|v| v.as_str()) {
        Some(nombre) if !nombre.trim().is_empty() => nombre,
        _ => return WsResponse::error(ctx.id, 400, "Error nombre no valido"),
    };

    let descripcion = match ctx.data.get("descripcion").and_then(|v| v.as_str()) {
        Some(nombre) => nombre,
        None => return WsResponse::error(ctx.id, 400, "Error descripcion no valida"),
    };

    let permisos_arr = match ctx.data.get("permisos").and_then(|v| v.as_array()) {
        Some(arr) if !arr.is_empty() => arr,
        _ => return WsResponse::error(ctx.id, 400, "Error permisos no validos"),
    };

    let permisos: Vec<String> = match permisos_arr
        .iter()
        .map(|v| v.as_str().map(String::from))
        .collect::<Option<Vec<String>>>()
    {
        Some(permisos) => permisos,
        None => return WsResponse::error(ctx.id, 400, "Error permisos no validos"),
    };

    let cargo_id = match ctx.data.get("cargo_id").and_then(|v| v.as_str()) {
        Some(cargo_id) => cargo_id,
        None => return WsResponse::error(ctx.id, 400, "Falta el cargo"),
    };

    let cargo_id: Uuid = match Uuid::parse_str(cargo_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "cargo_id no valido"),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_update", err),
    };

    let update_cargo = match update_cargo(&mut *tx, &nombre, &descripcion, cargo_id).await {
        Ok(cargo) => cargo,
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_update", err),
    };

    if let Err(err) = sync_cargo_permisos(&mut *tx, cargo_id, permisos.clone()).await {
        return WsResponse::internal_error(ctx.id, "cargos_update", err);
    }

    if let Err(err) = create_audit_log(
        &mut *tx,
        "cargo",
        update_cargo.id,
        "update",
        ctx.user_id,
        None,
        Some(serde_json::json!({ "nombre": nombre, "descripcion": descripcion, "permisos": permisos })),
    ).await {
        return WsResponse::internal_error(ctx.id, "cargos_update", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "cargos_update", err);
    }

    ctx.emit(
        "cargos",
        "update",
        serde_json::json!({ "data": update_cargo }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({}))
}

pub async fn cargos_delete(ctx: Ctx) -> WsResponse {
    let cargo_id = match ctx.data.get("cargo_id").and_then(|v| v.as_str()) {
        Some(cargo_id) => cargo_id,
        None => return WsResponse::error(ctx.id, 400, "Falta el cargo"),
    };

    let cargo_id: Uuid = match Uuid::parse_str(cargo_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "cargo_id no valido"),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "cargos_delete", err),
    };

    if let Err(err) = soft_delete_cargo(&mut *tx, cargo_id).await {
        return WsResponse::internal_error(ctx.id, "cargos_delete", err);
    }

    if let Err(err) = create_audit_log(
        &mut *tx,
        "cargo",
        cargo_id,
        "delete",
        ctx.user_id,
        None,
        Some(serde_json::json!({ "cargo_id": cargo_id, "active": false })),
    )
    .await
    {
        return WsResponse::internal_error(ctx.id, "cargos_delete", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "cargos_delete", err);
    }

    ctx.emit(
        "cargos",
        "delete",
        serde_json::json!({ "cargo_id": cargo_id }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({}))
}
