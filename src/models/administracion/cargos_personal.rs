use chrono::{DateTime, Utc};
use serde::Serialize;
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


#[derive(Debug)]
pub struct CargoPersonalAddPayload {
    pub nombre: String,
    pub tipo_contrato: String,
}

fn normalizar(
    nombre: &str,
    tipo_contrato: &str,
) -> Result<CargoPersonal, ValidacionError> {
    let nombre = nombre.trim();
    let tipo_contrato = tipo_contrato.trim();

    if nombre.is_empty() {
        return Err(ValidacionError::nuevo("El nombre no puede estar vacío"));
    }

    if tipo_contrato.is_empty() {
        return Err(ValidacionError::nuevo("El tipo de contrato no puede estar vacío"));
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
        normalizar(
            &self.nombre, 
            &self.tipo_contrato
        )
    }
}