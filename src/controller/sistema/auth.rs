use std::{collections::HashSet, sync::Arc};

use axum::{Json, extract::State, response::IntoResponse};
use chrono::Duration;
use uuid::Uuid;

use crate::{
    app::{app::AppState, error::ApiError},
    routes::protocol::{Ctx, WsResponse},
    security::password::hashear,
    services::{
        administracion::usuarios::self_newpassword_usuario,
        error::ServiceError,
        logs::audit_logs::create_audit_log,
        sistema::auth,
    },
};

#[derive(serde::Deserialize)]
pub struct LoginInput {
    pub usuario: String,
    pub password: String,
}

#[derive(serde::Deserialize)]
pub struct ChangePasswordInput {
    pub usuario: String,
    pub new_password: String,
}

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<impl IntoResponse, ApiError> {
    let LoginInput { usuario, password } = input;

    let Some(usuario) = auth::verificar_credenciales(&state.pool, &usuario, &password).await?
    else {
        return Err(ApiError::CredencialesInvalidas);
    };

    if !usuario.activo {
        return Err(ApiError::UsuarioInactivo);
    }

    let permisos = auth::get_permisos_por_cargo(&state.pool, usuario.cargo_id.clone()).await?;
    let permisos_map: Arc<HashSet<String>> = Arc::new(permisos.clone().into_iter().collect());

    let session = state.sessions.crear(
        usuario.id,
        usuario.cargo_id,
        Duration::minutes(480),
        permisos_map,
    )?;

    Ok(Json(
        serde_json::json!({ "status": "ok", "session_id": session, "permisos": permisos, "usuario": usuario }),
    ))
}

pub async fn list_usuarios(ctx: Ctx) -> WsResponse {
    println!(
        "[controller::sistema::auth] Listando Usuarios (id={})",
        ctx.id
    );
    WsResponse::ok(ctx.id, serde_json::Value::Null)
}

pub async fn logout(ctx: Ctx) -> WsResponse {
    let token = match ctx.data.get("token").and_then(|v| v.as_str()) {
        Some(token) => token,
        None => return WsResponse::error(ctx.id, 400, "No hay token"),
    };

    let token_uuid: Uuid = match Uuid::parse_str(token) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, "token invalido"),
    };

    if let Err(err) = ctx.state.sessions.eliminar(&token_uuid) {
        return WsResponse::internal_error(ctx.id, "auth:logout", err);
    }

    WsResponse::ok(ctx.id, serde_json::Value::Null)
}

pub async fn change_password(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<ChangePasswordInput>,
) -> Result<impl IntoResponse, ApiError> {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError::TokenAusente)?;

    let session_id = Uuid::parse_str(token).map_err(|_| ApiError::TokenInvalido)?;
    let session = state
        .sessions
        .validar(&session_id)?
        .ok_or(ApiError::TokenInvalido)?;

    let ChangePasswordInput { new_password, .. } = input;

    let hash = hashear(&new_password)?;

    let mut tx = state.pool.begin().await.map_err(ServiceError::from)?;

    let usuario = self_newpassword_usuario(&mut *tx, session.usuario_id, &hash).await?;

    create_audit_log(
        &mut *tx,
        "usuario",
        usuario.id,
        "cambiar_password",
        session.usuario_id,
        None,
        Some(serde_json::json!({ "accion": "cambiar_password" })),
    )
    .await?;

    tx.commit().await.map_err(ServiceError::from)?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}
