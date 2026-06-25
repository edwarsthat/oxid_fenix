use oxid_fenix::{
    app::app::{AppState, build_router}, 
    db::postgres::connect, 
    error::AppError, 
    sessions::memory::SessionStore
};


#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), AppError>{
    dotenvy::dotenv().ok();
    let pool = connect().await?;
    let sessions = SessionStore::new();
    let state = AppState { pool, sessions };
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    Ok(())
}