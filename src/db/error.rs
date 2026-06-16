use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConnectError {
    #[error("DATABASE_URL must be set")]
    MissingDatabaseUrl,

    #[error("failed to connect to Postgres: {0}")]
    Connection(#[from] sqlx::Error),
}
