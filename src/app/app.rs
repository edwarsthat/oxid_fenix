use sqlx::PgPool;
use axum::{Router, extract::State, routing::get};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
}

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .with_state(state)
}

async fn health(State(state): State<AppState>) -> &'static str {
    "ok"
}