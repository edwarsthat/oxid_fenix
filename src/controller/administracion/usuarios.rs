use uuid::Uuid;

use crate::{
    models::usuario::UsuariosAddPayload, routes::protocol::{Ctx, WsResponse}, security::password::{generar_temporal, hashear}, services::{administracion::usuarios::{create_usuario, get_usuarios}, logs::audit_logs::create_audit_log},
};

pub async fn usuarios_read(ctx: Ctx) -> WsResponse {
    match get_usuarios(&ctx.state.pool).await {
        Ok(usuarios) => WsResponse::ok(ctx.id, serde_json::json!({ "data": usuarios })),
        Err(err) => WsResponse::from_service_error(ctx.id, "usuarios_read", err),
    }
}

pub async fn usuarios_add(ctx: Ctx) -> WsResponse {
    let payload: UsuariosAddPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("payload invalido: {err}")),
        };

    if payload.nombre.trim().is_empty()
        || payload.apellido.trim().is_empty()
        || payload.email.trim().is_empty()
        || payload.usuario.trim().is_empty()
    {
        return WsResponse::error(ctx.id, 400, "hay campos vacios");
    }

    let cargo_id: Uuid = match Uuid::parse_str(&payload.cargo_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "cargo_id no valido"),
    };

    let password = generar_temporal();
    let hash = match hashear(&password) {
        Ok(hash) => hash,
        Err(err) => return WsResponse::internal_error(ctx.id, "usuarios_add", err),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "usuarios_add", err),
    };

    let nuevo_usuario = match create_usuario(
        &mut *tx,
        &payload.nombre,
        &payload.apellido,
        &payload.email,
        &payload.usuario,
        &hash,
        cargo_id,
    )
    .await
    {
        Ok(usuario) => usuario,
        Err(err) => return WsResponse::from_service_error(ctx.id, "usuarios_add", err),
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "usuario",
        nuevo_usuario.id,
        "add",
        ctx.user_id,
        None,
        Some(serde_json::json!({
            "nombre": payload.nombre,
            "apellido": payload.apellido,
            "email": payload.email,
            "usuario": payload.usuario,
            "cargo_id": cargo_id,
        })),
    ).await {
        return WsResponse::from_service_error(ctx.id, "usuarios_add", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "usuarios_add", err);
    }

    ctx.emit(
        "usuarios",
        "add",
        serde_json::json!({ "data": nuevo_usuario }),
    );

    WsResponse::ok(
        ctx.id,
        serde_json::json!({ "data": nuevo_usuario, "password_temporal": password }),
    )
}

pub async fn usuarios_update(ctx: Ctx) -> WsResponse {
    
}