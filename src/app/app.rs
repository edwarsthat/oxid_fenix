use axum::{
    Router,
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
};
use sqlx::PgPool;

use crate::{
    app::error::WsError,
    controller::sistema::auth::login,
    sessions::memory::{Session, SessionStore},
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub sessions: SessionStore,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/login", post(login))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn health(State(_): State<AppState>) -> &'static str {
    "ok"
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, WsError> {
    let token = match extraer_token(&headers) {
        Ok(token) => token,
        Err(err) => return Err(err),
    };

    let session = match resolver_session(&token, &state.sessions) {
        Ok(session) => session,
        Err(err) => return Err(err)
    };
    
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, session)))
}

fn extraer_token(headers: &HeaderMap) -> Result<&str, WsError> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(WsError::TokenAusente)
}

fn resolver_session(token: &str, sessions: &SessionStore) -> Result<Session, WsError> {
    let id = uuid::Uuid::parse_str(token).map_err(|_| WsError::TokenInvalido)?;
    let session = sessions.validar(&id)?.ok_or(WsError::TokenInvalido)?;
    Ok(session)
}

async fn handle_socket(mut socket: WebSocket, state: AppState, session: Session) {
    println!(
        "[ws] conexión establecida para usuario {} (cargo {})",
        session.usuario_id, session.cargo_id
    );

    use axum::extract::ws::Message;

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = &msg {
            println!("[ws] mensaje del cliente: {text}");
            
            let resp = crate::routes::dispatcher::dispatch(text, &state).await;
            let Ok(json) = serde_json::to_string(&resp) else {
                eprintln!("error serializando respuesta");
                continue;
            };
            if socket.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::{HeaderValue, header::AUTHORIZATION};
    use chrono::Duration;
    use std::collections::HashSet;
    use std::sync::Arc;
    use uuid::Uuid;

    fn headers_con_auth(valor: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(valor).unwrap());
        headers
    }

    fn permisos(items: &[&str]) -> Arc<HashSet<String>> {
        Arc::new(items.iter().map(|p| p.to_string()).collect())
    }

    // ── extraer_token ───────────────────────────────
    #[test]
    fn extraer_token_sin_header_es_ausente() {
        let err = extraer_token(&HeaderMap::new()).unwrap_err();
        assert!(matches!(err, WsError::TokenAusente));
    }

    #[test]
    fn extraer_token_sin_prefijo_bearer_es_ausente() {
        let headers = headers_con_auth("abc123"); // falta "Bearer "
        let err = extraer_token(&headers).unwrap_err();
        assert!(matches!(err, WsError::TokenAusente));
    }

    #[test]
    fn extraer_token_con_bearer_devuelve_el_token() {
        let headers = headers_con_auth("Bearer mi-token");
        assert_eq!(extraer_token(&headers).unwrap(), "mi-token");
    }

    // ── resolver_session ────────────────────────────
    #[test]
    fn resolver_session_token_no_uuid_es_invalido() {
        let sessions = SessionStore::new();
        let err = resolver_session("no-soy-uuid", &sessions).unwrap_err();
        assert!(matches!(err, WsError::TokenInvalido));
    }

    #[test]
    fn resolver_session_uuid_sin_sesion_es_invalido() {
        let sessions = SessionStore::new();
        let token = Uuid::new_v4().to_string();
        let err = resolver_session(&token, &sessions).unwrap_err();
        assert!(matches!(err, WsError::TokenInvalido));
    }

    #[test]
    fn resolver_session_expirada_es_invalido() {
        // una sesión vencida se trata igual que un token inválido
        let sessions = SessionStore::new();
        let id = sessions
            .crear(
                Uuid::new_v4(),
                Uuid::new_v4(),
                Duration::seconds(-1),
                permisos(&["usuarios.leer"]),
            )
            .unwrap();

        let err = resolver_session(&id.to_string(), &sessions).unwrap_err();

        assert!(matches!(err, WsError::TokenInvalido));
    }

    #[test]
    fn resolver_session_valida_devuelve_la_sesion() {
        let sessions = SessionStore::new();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();
        let permisos = permisos(&["usuarios.leer"]);
        let id = sessions
            .crear(usuario_id, cargo_id, Duration::hours(1), permisos.clone())
            .unwrap();

        let session = resolver_session(&id.to_string(), &sessions).unwrap();

        assert_eq!(session.usuario_id, usuario_id);
        assert_eq!(session.cargo_id, cargo_id);
        assert_eq!(session.permisos, permisos);
    }
}
