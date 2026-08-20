use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use uuid::Uuid;

use crate::models::{
    inventarios::llaves_nfc::uid_nfc,
    validations::{ValidacionError, Validar},
};

const MOTIVOS: [&str; 5] = ["devolucion", "perdida", "dannada", "retiro", "reemplazo"];

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
    pub motivo_devolucion: String,
}

#[derive(Debug)]
pub struct AsignacionQuitarLlaveData {
    pub asignacion_id: Uuid,
    pub motivo_devolucion: String,
    //service no toca llave_nfc.estado
    pub estado_llave: Option<&'static str>,
}

pub fn estado_por_motivo(motivo: &str) -> Option<&'static str> {
    match motivo {
        "perdida" => Some("perdida"),
        "dannada" => Some("dannada"),
        _ => None,
    }
}

pub fn motivo_devolucion(valor: &str) -> Result<String, ValidacionError> {
    let valor = valor.trim().to_lowercase();

    if !MOTIVOS.contains(&valor.as_str()) {
        return Err(ValidacionError::nuevo(format!(
            "el motivo debe ser uno de: {}",
            MOTIVOS.join(", ")
        )));
    }

    Ok(valor)
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
        let motivo_devolucion = motivo_devolucion(&self.motivo_devolucion)?;
        let estado_llave = estado_por_motivo(&motivo_devolucion);
        Ok(AsignacionQuitarLlaveData {
            asignacion_id: self.asignacion_id,
            motivo_devolucion,
            estado_llave,
        })
    }
}
