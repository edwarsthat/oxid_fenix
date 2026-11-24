use thiserror::Error;

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("no encontrado: {0}")]
    NotFound(String),

    #[error("conflicto: {0}")]
    Conflict(String),

    /// El registro cambió entre el read y el update. Va aparte de `Conflict`
    /// porque el cliente tiene que recargar, no corregir lo que escribió.
    #[error("version desactualizada: {0}")]
    VersionDesactualizada(String),

    /// La clave ya se usó para guardar otros datos. Va aparte de `Conflict`
    /// porque aquí sí hay un registro guardado: el de la primera vez.
    #[error("clave reutilizada: {0}")]
    ClaveReutilizada(String),
}
