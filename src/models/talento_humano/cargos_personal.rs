use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::ValidacionError;

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

impl CargoPersonalAddPayload {
    pub fn validar(&self) -> Result<CargoPersonal, ValidacionError> {
        normalizar(&self.nombre, &self.tipo_contrato)
    }
}

impl CargoPersonalUpdatePayload {
    pub fn validar(&self) -> Result<CargoPersonal, ValidacionError> {
        let cargo_id = match Uuid::parse_str(&self.cargo_id) {
            Ok(id) => id,
            Err(_) => {
                return Err(ValidacionError::nuevo("El cargo_id no es un UUID válido"));
            }
        };

        let mut cargo_personal = normalizar(&self.nombre, &self.tipo_contrato)?;
        cargo_personal.id = cargo_id;

        Ok(cargo_personal)
    }
}
