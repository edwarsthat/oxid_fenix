use crate::models::validations::{
    LIMITE_MAXIMO, Rango, RangoValidado, ValidacionError, Validar, limpiar_busqueda,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Tipos de documento aceptados por la columna `tipo_documento`.
// const TIPOS_DOCUMENTO: [&str; 8] = ["CC", "CE", "TI", "RC", "PA", "NIT", "PEP", "PPT"];

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

/// Lo que manda el cliente.
#[derive(Debug, Deserialize)]
pub struct PersonalReadPayload {
    pub activo: Option<bool>,
    pub cargo_id: Option<String>,
    pub busqueda: Option<String>,
    pub retiro: Option<Rango<NaiveDate>>,
}

/// Lo que devuelve validar() y consume el servicio.
#[derive(Debug)]
pub struct PersonalFiltros {
    pub activo: bool,
    pub cargo_id: Option<Uuid>,
    pub busqueda: Option<String>,
    pub retiro: Option<RangoValidado<NaiveDate>>,
    pub limite: i64,
}


#[derive(Debug, Deserialize)]
pub struct PersonalAddPayload {
    pub tipo_documento: String,
    pub documento: String,
    pub nombre: String,
    pub apellido: String,
    pub fecha_nacimiento: Option<NaiveDate>,
    pub telefono: Option<String>,
    pub cargo_id: Uuid,
    pub fecha_ingreso: NaiveDate,
}

impl Validar for PersonalReadPayload {
    type Datos = PersonalFiltros;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let activo = self.activo.unwrap_or(true);

        let cargo_id = match self.cargo_id {
            Some(texto) => match Uuid::parse_str(texto.trim()) {
                Ok(id) => Some(id),
                Err(_) => return Err(ValidacionError::nuevo("el cargo_id no es un UUID válido")),
            },
            None => None,
        };

        let busqueda = match self.busqueda {
            Some(texto) => limpiar_busqueda(&texto),
            None => None,
        };

        let retiro = match self.retiro {
            Some(rango) => Some(rango.validar()?),
            None => None,
        };

        let limite = LIMITE_MAXIMO;

        Ok(PersonalFiltros {
            activo,
            cargo_id,
            busqueda,
            retiro,
            limite,
        })
    }
}
