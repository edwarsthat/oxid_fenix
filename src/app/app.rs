use axum::{
    Router, extract::{
        State,
        ws::{WebSocket, WebSocketUpgrade},
    }, response::IntoResponse, routing::{get, post},
};
use sqlx::PgPool;

use crate::controller::sistema::auth::login;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/login", post(login))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> &'static str {
    "ok"
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    use axum::extract::ws::Message;

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = &msg {
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
