use std::{collections::HashSet, sync::Arc};

use uuid::Uuid;

use crate::{
    models::administracion::usuario::{UsuariosAddPayload, UsuariosUpdatePayload},
    routes::protocol::{Ctx, WsResponse},
    security::password::{generar_temporal, hashear},
    services::{
        administracion::usuarios::{
            activar_usuario, create_usuario, get_usuarios, newpassword_usuario,
            soft_delete_usuario, update_usuario,
        },
        logs::audit_logs::create_audit_log,
        sistema::auth::get_permisos_por_cargo,
    },
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

    let datos = match payload.validar() {
        Ok(datos) => datos,
        Err(err) => return WsResponse::from_validacion_error(ctx.id, err),
    };

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
        &datos.nombre,
        &datos.apellido,
        &datos.email,
        &datos.usuario,
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
    )
    .await
    {
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
    let payload: UsuariosUpdatePayload =
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

    let usuario_id: Uuid = match Uuid::parse_str(&payload.usuario_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "usuario_id no valido"),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "usuarios_update", err),
    };

    let usuario_actualizado = match update_usuario(
        &mut *tx,
        &payload.nombre,
        &payload.apellido,
        &payload.email,
        &payload.usuario,
        cargo_id,
        usuario_id,
    )
    .await
    {
        Ok(usuario) => usuario,
        Err(err) => return WsResponse::from_service_error(ctx.id, "usuarios_update", err),
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "usuario",
        usuario_actualizado.id,
        "update",
        ctx.user_id,
        None,
        Some(serde_json::json!({
            "nombre": payload.nombre,
            "apellido": payload.apellido,
            "email": payload.email,
            "usuario": payload.usuario,
            "cargo_id": cargo_id,
        })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "usuarios_update", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "usuarios_update", err);
    }

    match get_permisos_por_cargo(&ctx.state.pool, cargo_id).await {
        Ok(p) => {
            let permisos: Arc<HashSet<String>> = Arc::new(p.into_iter().collect());
            if let Err(err) = ctx
                .state
                .sessions
                .actualizar_permisos_por_usuario(usuario_id, cargo_id, permisos)
            {
                tracing::error!("[usuarios_update] no se refrescaron permisos: {err}");
            }
        }
        Err(err) => tracing::error!("[usuarios_update] no se refrescaron permisos: {err}"),
    }

    ctx.emit(
        "usuarios",
        "update",
        serde_json::json!({ "data": usuario_actualizado }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": usuario_actualizado }))
}

pub async fn usuarios_delete(ctx: Ctx) -> WsResponse {
    let usuario_id = match ctx.data.get("usuario_id").and_then(|v| v.as_str()) {
        Some(cargo_id) => cargo_id,
        None => return WsResponse::error(ctx.id, 400, "Falta el usuario id"),
    };

    let usuario_id: Uuid = match Uuid::parse_str(&usuario_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "Usuario invalido"),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "usuarios_delete", err),
    };

    if let Err(err) = soft_delete_usuario(&mut *tx, usuario_id).await {
        return WsResponse::from_service_error(ctx.id, "usuarios_delete", err);
    }

    if let Err(err) = create_audit_log(
        &mut *tx,
        "usuario",
        usuario_id,
        "delete",
        ctx.user_id,
        None,
        Some(serde_json::json!({ "usuario_id": usuario_id, "active": false })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "usuarios_delete", err);
    }

    if let Err(err) = ctx.state.sessions.eliminar_por_usuario(usuario_id) {
        return WsResponse::internal_error(ctx.id, "auth:logout", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "usuarios_delete", err);
    }

    ctx.emit(
        "usuarios",
        "delete",
        serde_json::json!({ "usuario_id": usuario_id }),
    );

    return WsResponse::ok(ctx.id, serde_json::json!({}));
}

pub async fn usuarios_new_password(ctx: Ctx) -> WsResponse {
    let usuario_id = match ctx.data.get("usuario_id").and_then(|v| v.as_str()) {
        Some(cargo_id) => cargo_id,
        None => return WsResponse::error(ctx.id, 400, "Falta el usuario id"),
    };

    let usuario_id: Uuid = match Uuid::parse_str(&usuario_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "Usuario invalido"),
    };

    let password = generar_temporal();
    let hash = match hashear(&password) {
        Ok(hash) => hash,
        Err(err) => return WsResponse::internal_error(ctx.id, "usuarios_newpassword", err),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "usuarios_newpassword", err),
    };

    let usuario = match newpassword_usuario(&mut *tx, usuario_id, &hash).await {
        Ok(usuario) => usuario,
        Err(err) => return WsResponse::from_service_error(ctx.id, "usuarios_newpassword", err),
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "usuario",
        usuario.id,
        "reset_password",
        ctx.user_id,
        None,
        Some(serde_json::json!({ "accion": "reset_password" })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "usuarios_newpassword", err);
    }

    if let Err(err) = ctx.state.sessions.eliminar_por_usuario(usuario_id) {
        return WsResponse::internal_error(ctx.id, "auth:logout", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "usuarios_newpassword", err);
    }

    return WsResponse::ok(ctx.id, serde_json::json!({"password_temporal": password }));
}

pub async fn usuarios_activar(ctx: Ctx) -> WsResponse {
    let usuario_id = match ctx.data.get("usuario_id").and_then(|v| v.as_str()) {
        Some(usuario_id) => usuario_id,
        None => return WsResponse::error(ctx.id, 400, "Falta el usuario id"),
    };

    let usuario_id: Uuid = match Uuid::parse_str(&usuario_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "usuario_id no valido"),
    };

    let password = generar_temporal();
    let hash = match hashear(&password) {
        Ok(hash) => hash,
        Err(err) => return WsResponse::internal_error(ctx.id, "usuarios_activar", err),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => return WsResponse::internal_error(ctx.id, "usuarios_activar", err),
    };

    let usuario_activado = match activar_usuario(&mut *tx, usuario_id, &hash).await {
        Ok(usuario) => usuario,
        Err(err) => return WsResponse::from_service_error(ctx.id, "usuarios_activar", err),
    };

    if let Err(err) = create_audit_log(
        &mut *tx,
        "usuario",
        usuario_activado.id,
        "activar",
        ctx.user_id,
        None,
        Some(serde_json::json!({ "usuario_id": usuario_id, "activo": true })),
    )
    .await
    {
        return WsResponse::from_service_error(ctx.id, "usuarios_activar", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "usuarios_activar", err);
    }

    ctx.emit(
        "usuarios",
        "update",
        serde_json::json!({ "data": usuario_activado }),
    );

    WsResponse::ok(
        ctx.id,
        serde_json::json!({ "data": usuario_activado, "password_temporal": password }),
    )
}
