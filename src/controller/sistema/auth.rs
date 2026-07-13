use std::{collections::HashSet, sync::Arc};

use axum::{Json, extract::State, response::IntoResponse};
use chrono::Duration;

use crate::{
    app::{app::AppState, error::ApiError},
    routes::protocol::{Ctx, WsResponse},
    services::sistema::auth,
};

#[derive(serde::Deserialize)]
pub struct LoginInput {
    pub usuario: String,
    pub password: String,
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

    let permisos = auth::get_permisos_por_cargo(&state.pool, usuario.cargo_id).await?;
    let permisos_map: Arc<HashSet<String>> = Arc::new(permisos.clone().into_iter().collect());

    let session = state
        .sessions
        .crear(usuario.id, usuario.cargo_id, Duration::minutes(480), permisos_map)?;

    Ok(Json(serde_json::json!({ "status": "ok", "session_id": session, "permisos": permisos })))
}

pub async fn list_usuarios(ctx: Ctx) -> WsResponse {
    println!(
        "[controller::sistema::auth] Listando Usuarios (id={})",
        ctx.id
    );
    WsResponse::ok(ctx.id, serde_json::Value::Null)
}
