use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use oxid_fenix::{
    app::app::{AppState, build_router},
    db::postgres::connect,
    error::AppError,
    sessions::memory::SessionStore,
};
use tokio::sync::broadcast;

const DEFAULT_BIND_ADDR: IpAddr = IpAddr::V4(std::net::Ipv4Addr::new(127, 0, 0, 1));
const DEFAULT_PORT: u16 = 3000;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), AppError> {
    tracing_subscriber::fmt::init();
    dotenvy::dotenv().ok();
    let pool = connect().await?;
    let sessions = SessionStore::new();
    sessions.iniciar_limpieza(Duration::from_secs(300));
    let (eventos_tx, _) = broadcast::channel(100);
    let state = AppState {
        pool,
        sessions,
        eventos: eventos_tx,
    };
    let app = build_router(state);

    let addr = SocketAddr::new(bind_addr()?, port()?);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("escuchando en http://{}", listener.local_addr()?);
    // `into_make_service_with_connect_info` es lo que hace llegar la IP del
    // cliente hasta el rate limit. Sin esto todas las peticiones caerían en una
    // única cubeta compartida.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}

fn bind_addr() -> Result<IpAddr, AppError> {
    let Ok(raw) = std::env::var("BIND_ADDR") else {
        return Ok(DEFAULT_BIND_ADDR);
    };
    raw.parse().map_err(|_| AppError::InvalidBindAddr(raw))
}

fn port() -> Result<u16, AppError> {
    let Ok(raw) = std::env::var("PORT") else {
        return Ok(DEFAULT_PORT);
    };
    raw.parse().map_err(|_| AppError::InvalidPort(raw))
}
