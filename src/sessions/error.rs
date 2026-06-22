
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("el lock de sesiones esta envenenado")]
    LockEnvenenado,
}