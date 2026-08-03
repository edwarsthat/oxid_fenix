use crate::db::error::ConnectError;
use sqlx::PgPool;

pub async fn connect() -> Result<PgPool, ConnectError> {
    let database_url =
        std::env::var("DATABASE_URL").map_err(|_| ConnectError::MissingDatabaseUrl)?;
    let pool = PgPool::connect(&database_url).await?;
    Ok(pool)
}
