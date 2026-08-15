use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{ValidacionError, Validar, texto_opcional};

const LARGOS_UID: [usize; 3] = [8, 14, 20];

#[derive(Debug, FromRow, Serialize)]
pub struct LlaveNfc {
    pub id: Uuid,
    pub uid: String,
    pub codigo: String,
    pub estado: String,
    pub descripcion: Option<String>,
    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct LlaveNfcAddPayload {
    pub uid: String,
    pub descripcion: Option<String>,
}

#[derive(Debug)]
pub struct LlaveNfcNueva {
    pub uid: String,
    pub descripcion: Option<String>,
}

pub fn uid_nfc(valor: &str) -> Result<String, ValidacionError> {
    let uid: String = valor
        .chars()
        .filter(|c| !matches!(c, ':' | '-' | ' '))
        .collect::<String>()
        .to_uppercase();
    if uid.is_empty() {
        return Err(ValidacionError::nuevo("el uid no puede estar vacio"));
    }

    if !uid.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ValidacionError::nuevo(
            "el uid debe ser hexadecimal (0-9, A-F)",
        ));
    }

    if !LARGOS_UID.contains(&uid.len()) {
        return Err(ValidacionError::nuevo(
            "el uid debe tener 8, 14 o 20 digitos",
        ));
    }

    Ok(uid)
}

impl Validar for LlaveNfcAddPayload {
    type Datos = LlaveNfcNueva;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let uid = uid_nfc(&self.uid)?;
        let descripcion = texto_opcional(self.descripcion.as_deref(), "descripcion", 200)?;

        Ok(LlaveNfcNueva { uid, descripcion })
    }
}
