use thiserror::Error;
use crate::db::error::ConnectError;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("database error: {0}")]
    Database(#[from] ConnectError),
}