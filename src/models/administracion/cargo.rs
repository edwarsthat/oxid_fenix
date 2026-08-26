use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{ValidacionError, Validar, uuid_requerido};

/// Payload de las operaciones que solo señalan a un cargo (leer sus permisos,
/// actualizarlo, eliminarlo).
#[derive(Debug, Deserialize)]
pub struct CargoIdPayload {
    pub cargo_id: Option<String>,
}

impl Validar for CargoIdPayload {
    type Datos = Uuid;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        uuid_requerido(self.cargo_id, "cargo_id")
    }
}

#[derive(Debug, FromRow, Serialize)]
pub struct Cargo {
    pub id: Uuid,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub creado_en: DateTime<Utc>,
    pub activo: bool,
}
