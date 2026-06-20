use thiserror::Error;

#[derive(Debug, Error)]
pub enum SeedError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("password hashing error: {0}")]
    Hash(#[from] argon2::password_hash::Error),
}