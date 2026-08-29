use std::collections::HashSet;

use oxid_fenix::db::postgres::connect;
use sqlx::migrate::Migrator;

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("[migrate] error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let pool = connect().await?;

    let tabla_existe: bool =
        sqlx::query_scalar("SELECT to_regclass('_sqlx_migrations') IS NOT NULL")
            .fetch_one(&pool)
            .await?;

    let previas: HashSet<i64> = if tabla_existe {
        sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations WHERE success")
            .fetch_all(&pool)
            .await?
            .into_iter()
            .collect()
    } else {
        HashSet::new()
    };

    MIGRATOR.run(&pool).await?;

    let mut nuevas = 0;
    for m in MIGRATOR.iter() {
        if previas.contains(&m.version) {
            println!("[migrate] ok {} {}", m.version, m.description);
        } else {
            println!("[migrate] ok {} {}", m.version, m.description);
            nuevas += 1;
        }
    }
    println!(
        "[migrate] {nuevas} nueva(s), {} en total",
        MIGRATOR.iter().count()
    );

    pool.close().await;
    Ok(())
}
