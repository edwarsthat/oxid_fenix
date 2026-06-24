use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
}
