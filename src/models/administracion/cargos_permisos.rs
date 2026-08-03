use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize)]
pub struct CargosPermisos {
    pub cargo_id: Uuid,
    pub permiso_id: Uuid,
}
