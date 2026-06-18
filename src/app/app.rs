use sqlx::PgPool;
use axum::{Router, extract::{State, ws::{WebSocket, WebSocketUpgrade}}, response::IntoResponse, routing::get};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> &'static str {
    "ok"
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    use axum::extract::ws::Message;

    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = &msg {
            println!("cliente: {text}");
        }
        let response = match &msg {
            Message::Text(text) if text.trim() == "health" => Message::Text("ok".into()),
            other => other.clone(),
        };
        if socket.send(response).await.is_err() {
            break;
        }
    }
}