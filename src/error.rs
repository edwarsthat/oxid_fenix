use crate::db::error::ConnectError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] ConnectError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("BIND_ADDR must be a valid IP address, got '{0}'")]
    InvalidBindAddr(String),

    #[error("PORT must be a number between 0 and 65535, got '{0}'")]
    InvalidPort(String),
}
