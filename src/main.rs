use oxid_fenix::{db::postgres::connect, error::AppError};


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
    println!("Connected postgres");
    Ok(())
}