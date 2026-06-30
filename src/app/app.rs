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
    app::error::WsError, controller::sistema::auth::login, sessions::memory::{Session, SessionStore},
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
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(WsError::TokenAusente)?;

    let id = uuid::Uuid::parse_str(token).map_err(|_| WsError::TokenInvalido)?;
    
    let session = state
        .sessions
        .validar(&id)?
        .ok_or(WsError::TokenInvalido)?;

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, session)))
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
