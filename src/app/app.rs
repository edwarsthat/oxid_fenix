use std::sync::Arc;

use axum::{
    Router,
    extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    http::HeaderMap,
    middleware,
    response::IntoResponse,
    routing::{get, post},
};
use sqlx::PgPool;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    app::error::WsError,
    controller::sistema::auth::{change_password, login},
    routes::protocol::Evento,
    security::rate_limit::{RateLimiter, limitar},
    sessions::memory::{Session, SessionStore},
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub sessions: SessionStore,
    pub eventos: broadcast::Sender<Arc<Evento>>,
}

/// Límite por IP de cada endpoint, en (pico de golpe, ritmo sostenido por minuto).
///
/// El pico es lo que se tolera de una tacada con la cubeta llena; el ritmo es lo
/// que queda una vez agotado. Los dos números conviven: un cliente puede gastar
/// el pico entero y después sigue pasando al ritmo sostenido.
mod limites {
    /// Es el endpoint más barato del servicio y lo consulta el monitoreo, así
    /// que va holgado; el tope está para que no sirva de ping flood gratis.
    pub const HEALTH: (u32, u32) = (60, 60);

    /// `login` y `cambiar-password` pagan un argon2 por intento (~50-100 ms de
    /// CPU) y son el blanco natural de la fuerza bruta: van al ritmo más bajo.
    /// `cambiar-password` gasta dos hashes por llamada (verifica el actual y
    /// hashea el nuevo), de ahí que no sea más permisivo que el login.
    pub const LOGIN: (u32, u32) = (5, 5);
    pub const CAMBIAR_PASSWORD: (u32, u32) = (5, 5);

    /// Esto limita el handshake del websocket, no los mensajes de un socket ya
    /// abierto. Deja margen para reconexiones cuando se cae la red, pero corta
    /// el ciclo de abrir y cerrar conexiones sin parar.
    pub const WS: (u32, u32) = (20, 20);
}

/// Cada endpoint estrena su propia cubeta: gastar el cupo de `/health` no puede
/// dejar a nadie fuera de `/login`.
macro_rules! rate_limit {
    ($limite:expr) => {{
        let (pico, por_minuto) = $limite;
        middleware::from_fn_with_state(RateLimiter::por_minuto(pico, por_minuto), limitar)
    }};
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health).layer(rate_limit!(limites::HEALTH)))
        .route("/login", post(login).layer(rate_limit!(limites::LOGIN)))
        .route(
            "/cambiar-password",
            post(change_password).layer(rate_limit!(limites::CAMBIAR_PASSWORD)),
        )
        .route("/ws", get(ws_handler).layer(rate_limit!(limites::WS)))
        // OJO: el orden importa. `.layer()` sobre el `MethodRouter` deja el
        // límite por delante del handler, así que un login de más se corta
        // antes de tocar la base de datos y antes de pagar el argon2.
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

    let (session_id, session) = match resolver_session(&token, &state.sessions) {
        Ok(par) => par,
        Err(err) => return Err(err),
    };

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, session_id, session)))
}

fn extraer_token(headers: &HeaderMap) -> Result<&str, WsError> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(WsError::TokenAusente)
}

/// Devuelve también el id de la sesión: el socket lo necesita para revalidar
/// permisos en cada evento, ya que la copia de `Session` se congela al conectar.
fn resolver_session(token: &str, sessions: &SessionStore) -> Result<(Uuid, Session), WsError> {
    let id = Uuid::parse_str(token).map_err(|_| WsError::TokenInvalido)?;
    let session = sessions.validar(&id).ok_or(WsError::TokenInvalido)?;
    Ok((id, session))
}

async fn handle_socket(socket: WebSocket, state: AppState, session_id: Uuid, session: Session) {
    println!(
        "[ws] conexión establecida para usuario {} (cargo {})",
        session.usuario_id, session.cargo_id
    );

    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();
    let mut eventos_rx = state.eventos.subscribe();

    loop {
        tokio::select! {
        msg = receiver.next() => {
            match msg {
                Some(Ok(Message::Text(text))) => {
                     let resp = crate::routes::dispatcher::dispatch(&text, &state).await;
                     let Ok(json) = serde_json::to_string(&resp) else { continue };
                     if sender.send(Message::Text(json.into())).await.is_err() { break; }
                }
                Some(Ok(_)) => {}
                Some(Err(e)) => {
                    eprintln!("Error leyendo del socket: {e}");
                    break;
                }
                None => {
                    // El stream terminó, el cliente cerró la conexión
                    break;
                }
            }
        }

        evento = eventos_rx.recv() => {
            match evento {
                Ok(ev) => {
                    // la sesión pudo revocarse o cambiar de permisos con el socket ya abierto
                    let Some(sesion) = state.sessions.validar(&session_id) else { break };
                    if sesion.debe_cambiar_password { continue; }
                    if !sesion.permisos.contains(&ev.permiso) { continue; }
                    if sender.send(Message::Text(ev.json.clone().into())).await.is_err() { break; }
                }
                Err(broadcast::error::RecvError::Lagged(_)) => continue,
                Err(_) => break,
            }
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
        let id = sessions.crear(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Duration::seconds(-1),
            permisos(&["usuarios.leer"]),
            false,
        );

        let err = resolver_session(&id.to_string(), &sessions).unwrap_err();

        assert!(matches!(err, WsError::TokenInvalido));
    }

    #[test]
    fn resolver_session_valida_devuelve_la_sesion() {
        let sessions = SessionStore::new();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();
        let permisos = permisos(&["usuarios.leer"]);
        let id = sessions.crear(
            usuario_id,
            cargo_id,
            Duration::hours(1),
            permisos.clone(),
            false,
        );

        let (session_id, session) = resolver_session(&id.to_string(), &sessions).unwrap();

        assert_eq!(session_id, id);
        assert_eq!(session.usuario_id, usuario_id);
        assert_eq!(session.cargo_id, cargo_id);
        assert_eq!(session.permisos, permisos);
    }
}
