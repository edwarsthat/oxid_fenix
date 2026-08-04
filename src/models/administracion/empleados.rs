use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize)]
pub struct Empleado {
    pub id: Uuid,
    pub codigo: String,
    pub tipo_documento: String,
    pub documento: String,
    pub nombre: String,
    pub apellido: String,
    pub fecha_nacimiento: Option<NaiveDate>,
    pub telefono: Option<String>,
    pub cargo_id: Uuid,
    pub fecha_ingreso: NaiveDate,
    pub fecha_retiro: Option<NaiveDate>,
    pub activo: bool,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}
