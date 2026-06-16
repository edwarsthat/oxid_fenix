use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, FromRow)]
pub struct Permiso {
    pub id: Uuid,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub creado_en: DateTime<Utc>
}
