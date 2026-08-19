use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::models::{
    inventarios::llaves_nfc::uid_nfc,
    validations::{ValidacionError, Validar},
};

#[derive(Debug, FromRow, Serialize)]
pub struct AsignacionLlave {
    pub id: Uuid,

    pub llave_id: Uuid,
    pub empleado_id: Uuid,

    pub asignada_en: DateTime<Utc>,
    pub devuelta_en: Option<DateTime<Utc>>,
    pub motivo_devolucion: Option<String>,

    pub creado_en: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct AsignacionLlaveAddPayload {
    pub empleado_id: Uuid,
    pub uid: String,
}

#[derive(Debug)]
pub struct AsignacionLlaveNueva {
    pub empleado_id: Uuid,
    pub uid: String,
}

#[derive(Debug, Deserialize)]
pub struct AsignacionQuitarLlavePayload {
    pub asignacion_id: Uuid,
}

#[derive(Debug)]
pub struct AsignacionQuitarLlaveData {
    pub asignacion_id: Uuid,
}

impl Validar for AsignacionLlaveAddPayload {
    type Datos = AsignacionLlaveNueva;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let uid = uid_nfc(&self.uid)?;

        Ok(AsignacionLlaveNueva {
            uid,
            empleado_id: self.empleado_id,
        })
    }
}

impl Validar for AsignacionQuitarLlavePayload {
    type Datos = AsignacionQuitarLlaveData;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        Ok(AsignacionQuitarLlaveData {
            asignacion_id: self.asignacion_id,
        })
    }
}
