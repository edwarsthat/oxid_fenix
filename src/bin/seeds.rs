
use oxid_fenix::{db::postgres::connect, seeds};

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("[seed] error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let pool = connect().await?;
    seeds::run_all(&pool).await?;
    Ok(())
}
