
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("el lock de sesiones esta envenenado")]
    LockEnvenenado,

    #[error("la sesión expiró")]
    Expired,

    #[error("la sesión fue revocada")]
    Revoked,

    #[error("sesión no encontrada")]
    NotFound,
}