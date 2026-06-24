use axum::{Json, extract::State, response::IntoResponse};

use crate::{
    app::{app::AppState, error::ApiError}, 
    error::AppError, routes::protocol::{Ctx, WsResponse}, 
    services::sistema::auth
};

#[derive(serde::Deserialize)]
pub struct LoginInput {
    pub usuario: String,
    pub password: String
}

pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginInput>,
) -> Result<impl IntoResponse, ApiError>{

    let LoginInput { usuario, password} = input;
    let usuario = auth::get_usuario_username(&state.pool, &usuario).await?;
    
    println!("{:?}", usuario);
    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub async fn list_usuarios(ctx: Ctx) -> WsResponse {
    println!("[controller::sistema::auth] Listando Usuarios (id={})", ctx.id);
    WsResponse::ok(ctx.id, serde_json::Value::Null)
}