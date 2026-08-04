use std::{collections::HashSet, sync::Arc};

use axum::{Json, extract::State, response::IntoResponse};
use chrono::Duration;
use uuid::Uuid;

use crate::{
    app::{app::AppState, error::ApiError},
    models::{
        auth::login::{ChangePasswordInput, LoginInput},
        validations::Validar,
    },
    routes::protocol::{Ctx, WsResponse},
    security::password::hashear,
    services::{
        administracion::usuarios::self_newpassword_usuario, error::ServiceError,
        logs::audit_logs::create_audit_log, sistema::auth,
    },
};

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

    // La sesión arrastra el flag: el dispatcher la deja solo cerrar sesión hasta
    // que la contraseña temporal se cambie por /cambiar-password.
    let session = state.sessions.crear(
        usuario.id,
        usuario.cargo_id,
        Duration::minutes(480),
        permisos_map,
        usuario.debe_cambiar_password,
    );

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
    let token_uuid: Uuid = match Uuid::parse_str(&ctx.token) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 401, "no autenticado"),
    };

    if ctx.state.sessions.validar(&token_uuid).is_none() {
        return WsResponse::error(ctx.id, 401, "no autenticado");
    }

    ctx.state.sessions.eliminar(&token_uuid);

    WsResponse::ok(ctx.id, serde_json::Value::Null)
}

pub async fn change_password(
    State(state): State<AppState>,
    headers: axum::http::HeaderMap,
    Json(input): Json<ChangePasswordInput>,
) -> Result<impl IntoResponse, ApiError> {
    let datos = input.validar()?;

    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(ApiError::TokenAusente)?;

    // La sesión primero: verificar credenciales cuesta un argon2 (~50-100ms), no
    // conviene gastarlo antes de saber que quien llama tiene una sesión viva.
    let session_id = Uuid::parse_str(token).map_err(|_| ApiError::TokenInvalido)?;
    let session = state
        .sessions
        .validar(&session_id)
        .ok_or(ApiError::TokenInvalido)?;

    let Some(autenticado) =
        auth::verificar_credenciales(&state.pool, &datos.usuario, &datos.password_actual).await?
    else {
        return Err(ApiError::CredencialesInvalidas);
    };

    // El usuario del payload tiene que ser el dueño de la sesión. Sin esto,
    // cualquiera con una cuenta válida cambia la contraseña de otro mandando sus
    // propias credenciales junto al token ajeno.
    if autenticado.id != session.usuario_id {
        return Err(ApiError::CredencialesInvalidas);
    }

    let hash = hashear(&datos.password_nueva)?;
    let mut tx = state.pool.begin().await.map_err(ServiceError::from)?;

    let usuario = self_newpassword_usuario(&mut *tx, session.usuario_id, &hash).await?;

    create_audit_log(
        &mut *tx,
        "usuarios",
        usuario.id,
        "cambiar_password",
        session.usuario_id,
        None,
        Some(serde_json::json!({ "accion": "cambiar_password" })),
    )
    .await?;

    tx.commit().await.map_err(ServiceError::from)?;

    // Las sesiones vivas son una foto del login y siguen con el flag en true, lo
    // que dejaría al usuario bloqueado en bucle. Se cierran para que vuelva a
    // entrar con la contraseña nueva.
    state.sessions.eliminar_por_usuario(session.usuario_id);

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::memory::SessionStore;
    use sqlx::PgPool;
    use tokio::sync::broadcast;

    fn state_de_prueba() -> AppState {
        let pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        AppState {
            pool,
            sessions: SessionStore::new(),
            eventos: broadcast::Sender::new(100),
        }
    }

    fn crear_sesion(state: &AppState) -> Uuid {
        state.sessions.crear(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Duration::hours(1),
            Arc::new(HashSet::new()),
            false,
        )
    }

    fn ctx(state: &AppState, token: &str, data: serde_json::Value) -> Ctx {
        Ctx {
            state: state.clone(),
            id: "id-1".into(),
            user_id: Uuid::nil(),
            data: data.as_object().cloned().unwrap_or_default(),
            token: token.into(),
            permisos: Arc::new(HashSet::new()),
        }
    }

    #[tokio::test]
    async fn logout_cierra_la_sesion_del_token_de_la_peticion() {
        let state = state_de_prueba();
        let token = crear_sesion(&state);

        let resp = logout(ctx(&state, &token.to_string(), serde_json::Value::Null)).await;

        assert_eq!(resp.status, 200);
        assert!(state.sessions.validar(&token).is_none());
    }

    #[tokio::test]
    async fn logout_no_cierra_la_sesion_de_otro_desde_el_payload() {
        let state = state_de_prueba();
        let victima = crear_sesion(&state);
        let atacante = crear_sesion(&state);

        // el atacante manda su propio token y el id de sesión ajeno en el payload
        let resp = logout(ctx(
            &state,
            &atacante.to_string(),
            serde_json::json!({ "token": victima.to_string() }),
        ))
        .await;

        assert_eq!(resp.status, 200);
        assert!(state.sessions.validar(&victima).is_some());
        assert!(state.sessions.validar(&atacante).is_none());
    }

    #[tokio::test]
    async fn logout_sin_sesion_devuelve_401() {
        let state = state_de_prueba();
        let victima = crear_sesion(&state);

        // sin token válido, aunque conozca el id de sesión de la víctima
        let resp = logout(ctx(
            &state,
            "sin-sesion",
            serde_json::json!({ "token": victima.to_string() }),
        ))
        .await;

        assert_eq!(resp.status, 401);
        assert!(state.sessions.validar(&victima).is_some());
    }

    #[tokio::test]
    async fn logout_con_token_inexistente_devuelve_401() {
        let state = state_de_prueba();

        let resp = logout(ctx(
            &state,
            &Uuid::new_v4().to_string(),
            serde_json::Value::Null,
        ))
        .await;

        assert_eq!(resp.status, 401);
    }
}
