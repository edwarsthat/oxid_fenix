use serde::Serialize;
use uuid::Uuid;
use sqlx::FromRow;


#[derive(Debug, FromRow, Serialize)]
pub struct CargosPermisos {
    pub cargo_id: Uuid,
    pub permiso_id: Uuid,
}