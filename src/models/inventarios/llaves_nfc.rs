use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{
    ValidacionError, Validar, limitar, limpiar_busqueda, texto_opcional,
};

const LARGOS_UID: [usize; 3] = [8, 14, 20];

/// Estados válidos de `llaves_nfc.estado`. Es la misma lista del CHECK de la
/// migración: si cambia una, cambia la otra.
const ESTADOS: [&str; 4] = ["inventario", "perdida", "dañada", "baja"];

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

/// Lo que manda el cliente.
#[derive(Debug, Deserialize)]
pub struct LlaveNfcReadPayload {
    pub estado: Option<String>,
    pub busqueda: Option<String>,
    pub uid: Option<String>,
    pub limite: Option<i64>,
}

/// Lo que devuelve validar() y consume el servicio.
#[derive(Debug)]
pub struct LlaveNfcFiltros {
    pub estado: Option<String>,
    pub busqueda: Option<String>,
    pub uid: Option<String>,
    pub limite: i64,
}

#[derive(Debug, Deserialize)]
pub struct LlaveNfcUpdatePayload {
    pub llave_nfc_id: Uuid,
    pub estado: String,
    pub descripcion: String,
    pub version: i32,
}

/// Lo que el update puede tocar. El uid no está a propósito: identifica la
/// tarjeta física y cambiarlo sería registrar otra llave, no editar esta.
#[derive(Debug)]
pub struct LlaveNfcCambios {
    pub estado: String,
    pub descripcion: Option<String>,
}

#[derive(Debug)]
pub struct LlaveNfcActualizado {
    pub llave_nfc_id: Uuid,
    pub version: i32,
    pub datos: LlaveNfcCambios,
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

/// Normaliza y comprueba el estado contra la misma lista del CHECK. En el
/// UPDATE la tabla igual lo atraparía, pero como 23514 genérico: validarlo acá
/// deja el mensaje con los valores que sí sirven.
pub fn estado_nfc(valor: &str) -> Result<String, ValidacionError> {
    let valor = valor.trim().to_lowercase();

    if !ESTADOS.contains(&valor.as_str()) {
        return Err(ValidacionError::nuevo(format!(
            "el estado debe ser uno de: {}",
            ESTADOS.join(", ")
        )));
    }

    Ok(valor)
}

impl Validar for LlaveNfcAddPayload {
    type Datos = LlaveNfcNueva;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let uid = uid_nfc(&self.uid)?;
        let descripcion = texto_opcional(self.descripcion.as_deref(), "descripcion", 200)?;

        Ok(LlaveNfcNueva { uid, descripcion })
    }
}

impl Validar for LlaveNfcReadPayload {
    type Datos = LlaveNfcFiltros;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // Un estado en blanco es "sin filtro"; uno mal escrito es un error y no
        // un listado vacío: en un SELECT el CHECK de la tabla no atrapa nada.
        let estado = match self.estado.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(valor) => Some(estado_nfc(valor)?),
        };

        // Filtro de texto, no columna a guardar: va por `limpiar_busqueda` para
        // que un '%' escrito por el usuario no se lea como comodín del LIKE.
        let busqueda = match self.busqueda {
            Some(texto) => limpiar_busqueda(&texto),
            None => None,
        };

        // El uid es UNIQUE: acá filtra por igualdad exacta, normalizado igual
        // que en el alta (mayúsculas, sin ':' ni '-'), para que buscar la
        // tarjeta que se acaba de leer con el lector siempre acierte.
        let uid = match self.uid.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(valor) => Some(uid_nfc(valor)?),
        };

        let limite = limitar(self.limite);

        Ok(LlaveNfcFiltros {
            estado,
            busqueda,
            uid,
            limite,
        })
    }
}

impl Validar for LlaveNfcUpdatePayload {
    type Datos = LlaveNfcActualizado;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // La tabla arranca en 1 y solo sube; un valor menor no salió de un read.
        if self.version < 1 {
            return Err(ValidacionError::nuevo(
                "La version de la llave no es valida, recarga los datos",
            ));
        }

        let llave_nfc_id = self.llave_nfc_id;
        let version = self.version;

        let estado = estado_nfc(&self.estado)?;

        // El update manda la descripción completa (reemplaza, no parchea), así
        // que mandarla en blanco es la forma de borrarla: queda NULL, igual que
        // en el alta.
        let descripcion = texto_opcional(Some(&self.descripcion), "descripcion", 200)?;

        Ok(LlaveNfcActualizado {
            llave_nfc_id,
            version,
            datos: LlaveNfcCambios {
                estado,
                descripcion,
            },
        })
    }
}
