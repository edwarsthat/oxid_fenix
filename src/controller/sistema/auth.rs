use axum::{Json, extract::State, response::IntoResponse};
use chrono::Duration;

use crate::{
    app::{app::AppState, error::ApiError},
    routes::protocol::{Ctx, WsResponse},
    security::password,
    services::sistema::auth,
};

#[derive(serde::Deserialize)]
pub struct LoginInput {
    pub usuario: String,
    pub password: String,
}

// Hash argon2 de cualquier contraseña, generado una vez con hashear()
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$vBUueYcnLVdWEHGAJXkFjQ$XnRTL9GMlDT4os2lmvc2WJTFH29bXPNZtbJ8n51bw2d";

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<impl IntoResponse, ApiError> {
    let LoginInput { usuario, password } = input;

    let usuario_opt = auth::get_usuario_username(&state.pool, &usuario).await?;

    let hash = usuario_opt
        .as_ref()
        .map(|u| u.password_hash.as_str())
        .unwrap_or(DUMMY_HASH);

    let is_correct = password::verificar(&password, hash).unwrap_or(false);

    let Some(usuario) = usuario_opt.filter(|_| is_correct) else {
        return Err(ApiError::CredencialesInvalidas);
    };

    let permisos = auth::get_permisos_por_cargo(&state.pool, usuario.cargo_id).await?;

    let session = state
        .sessions
        .crear(usuario.id, usuario.cargo_id, Duration::minutes(480))?;

    Ok(Json(serde_json::json!({ "status": "ok", "session_id": session, "permisos": permisos })))
}

pub async fn list_usuarios(ctx: Ctx) -> WsResponse {
    println!(
        "[controller::sistema::auth] Listando Usuarios (id={})",
        ctx.id
    );
    WsResponse::ok(ctx.id, serde_json::Value::Null)
}
