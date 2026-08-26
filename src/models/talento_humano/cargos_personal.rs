use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{ValidacionError, Validar, uuid_obligatorio, uuid_requerido};

#[derive(Debug, FromRow, Serialize)]
pub struct CargoPersonal {
    pub id: Uuid,
    pub nombre: String,
    pub tipo_contrato: String,
    pub creado_en: DateTime<Utc>,
    pub activo: bool,
}

#[derive(Debug, Deserialize)]
pub struct CargoPersonalAddPayload {
    pub nombre: String,
    pub tipo_contrato: String,
}

#[derive(Debug, Deserialize)]
pub struct CargoPersonalUpdatePayload {
    pub nombre: String,
    pub tipo_contrato: String,
    pub cargo_id: String,
}

fn normalizar(nombre: &str, tipo_contrato: &str) -> Result<CargoPersonal, ValidacionError> {
    let nombre = nombre.trim();
    let tipo_contrato = tipo_contrato.trim();

    if nombre.is_empty() {
        return Err(ValidacionError::nuevo("El nombre no puede estar vacío"));
    }

    if tipo_contrato.is_empty() {
        return Err(ValidacionError::nuevo(
            "El tipo de contrato no puede estar vacío",
        ));
    }

    Ok(CargoPersonal {
        id: Uuid::new_v4(),
        nombre: nombre.to_string(),
        tipo_contrato: tipo_contrato.to_string(),
        creado_en: Utc::now(),
        activo: true,
    })
}

/// Payload de las operaciones que solo señalan a un cargo (eliminar, activar).
#[derive(Debug, Deserialize)]
pub struct CargoPersonalIdPayload {
    pub cargo_id: Option<String>,
}

impl Validar for CargoPersonalIdPayload {
    type Datos = Uuid;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        uuid_requerido(self.cargo_id, "cargo_id")
    }
}

impl CargoPersonalAddPayload {
    pub fn validar(&self) -> Result<CargoPersonal, ValidacionError> {
        normalizar(&self.nombre, &self.tipo_contrato)
    }
}

impl CargoPersonalUpdatePayload {
    pub fn validar(&self) -> Result<CargoPersonal, ValidacionError> {
        let cargo_id = uuid_obligatorio(&self.cargo_id, "cargo_id")?;

        let mut cargo_personal = normalizar(&self.nombre, &self.tipo_contrato)?;
        cargo_personal.id = cargo_id;

        Ok(cargo_personal)
    }
}
