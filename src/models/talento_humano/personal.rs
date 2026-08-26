use crate::models::validations::{
    LIMITE_MAXIMO, Rango, RangoValidado, ValidacionError, Validar, limpiar_busqueda,
    texto_obligatorio, texto_opcional, uuid_obligatorio, uuid_opcional, uuid_requerido,
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Tipos de documento aceptados por la columna `tipo_documento`.
const TIPOS_DOCUMENTO: [&str; 8] = ["CC", "CE", "TI", "RC", "PA", "NIT", "PEP", "PPT"];

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
    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,

    //El id de la asignacion
    pub asignacion_id: Option<Uuid>,
    pub llave_id: Option<Uuid>,
    pub llave_codigo: Option<String>,
    pub llave_uid: Option<String>,
    pub asignado_en: Option<DateTime<Utc>>,
}

/// Sobre qué columna de fecha aplica el rango `fecha`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub enum TipoFecha {
    #[default]
    Ingreso,
    Retiro,
}

/// Lo que manda el cliente.
#[derive(Debug, Deserialize)]
pub struct PersonalReadPayload {
    pub activo: Option<bool>,
    pub empleados_id: Option<Vec<String>>,
    pub cargo_id: Option<String>,
    pub busqueda: Option<String>,
    pub fecha: Option<Rango<NaiveDate>>,
    pub tipo_fecha: Option<String>,
}

/// Lo que devuelve validar() y consume el servicio.
#[derive(Debug)]
pub struct PersonalFiltros {
    pub activo: bool,
    pub empleados_id: Option<Vec<Uuid>>,
    pub cargo_id: Option<Uuid>,
    pub busqueda: Option<String>,
    pub fecha: Option<RangoValidado<NaiveDate>>,
    pub tipo_fecha: TipoFecha,
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

#[derive(Debug)]
pub struct PersonalNuevo {
    pub tipo_documento: String,
    pub documento: String,
    pub nombre: String,
    pub apellido: String,
    pub fecha_nacimiento: Option<NaiveDate>,
    pub telefono: Option<String>,
    pub cargo_id: Uuid,
    pub fecha_ingreso: NaiveDate,
}

#[derive(Debug, Deserialize)]
pub struct PersonalUpdatePayload {
    pub empleado_id: Uuid,
    pub tipo_documento: String,
    pub documento: String,
    pub nombre: String,
    pub apellido: String,
    pub fecha_nacimiento: Option<NaiveDate>,
    pub telefono: Option<String>,
    pub cargo_id: Uuid,
    pub fecha_ingreso: NaiveDate,
    pub version: i32,
}

#[derive(Debug)]
pub struct PersonalActualizado {
    pub empleado_id: Uuid,
    pub version: i32,
    pub datos: PersonalNuevo,
}

/// Las operaciones que solo necesitan señalar a un empleado (retirar, activar)
/// comparten payload: el id llega como texto y se valida igual que el resto.
#[derive(Debug, Deserialize)]
pub struct PersonalIdPayload {
    pub empleado_id: Option<String>,
}

impl Validar for PersonalIdPayload {
    type Datos = Uuid;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        uuid_requerido(self.empleado_id, "empleado_id")
    }
}

impl Validar for PersonalReadPayload {
    type Datos = PersonalFiltros;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let activo = self.activo.unwrap_or(true);

        // Una lista vacía se trata como "sin filtro": `id = ANY('{}')` no
        // devolvería nada, y quien manda [] no está pidiendo cero empleados.
        let empleados_id = match self.empleados_id {
            Some(lista) if !lista.is_empty() => {
                let ids = lista
                    .into_iter()
                    .map(|texto| uuid_obligatorio(&texto, "empleados_id"))
                    .collect::<Result<Vec<_>, _>>()?;
                Some(ids)
            }
            _ => None,
        };

        let cargo_id = uuid_opcional(self.cargo_id, "cargo_id")?;

        let busqueda = match self.busqueda {
            Some(texto) => limpiar_busqueda(&texto),
            None => None,
        };

        let fecha = match self.fecha {
            Some(rango) => Some(rango.validar()?),
            None => None,
        };

        let tipo_fecha = match self.tipo_fecha.as_deref().map(str::trim) {
            None | Some("") | Some("ingreso") => TipoFecha::Ingreso,
            Some("retiro") => TipoFecha::Retiro,
            Some(_) => {
                return Err(ValidacionError::nuevo(
                    "el tipo_fecha debe ser 'ingreso' o 'retiro'",
                ));
            }
        };

        let limite = LIMITE_MAXIMO;

        Ok(PersonalFiltros {
            activo,
            empleados_id,
            cargo_id,
            busqueda,
            fecha,
            tipo_fecha,
            limite,
        })
    }
}

impl Validar for PersonalAddPayload {
    type Datos = PersonalNuevo;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let tipo_documento = self.tipo_documento.trim().to_uppercase();

        if !TIPOS_DOCUMENTO.contains(&tipo_documento.as_str()) {
            return Err(ValidacionError::nuevo(format!(
                "el tipo_documento debe ser uno de: {}",
                TIPOS_DOCUMENTO.join(", ")
            )));
        }

        let documento = texto_obligatorio(&self.documento, "documento", 30)?;
        let nombre = texto_obligatorio(&self.nombre, "nombre", 100)?;
        let apellido = texto_obligatorio(&self.apellido, "apellido", 100)?;
        let telefono = texto_opcional(self.telefono.as_deref(), "teléfono", 30)?;

        // La tabla solo verifica retiro >= ingreso; esto no lo cubre nadie más.
        if let Some(nacimiento) = self.fecha_nacimiento
            && nacimiento >= self.fecha_ingreso
        {
            return Err(ValidacionError::nuevo(
                "la fecha de nacimiento debe ser anterior a la fecha de ingreso",
            ));
        }

        Ok(PersonalNuevo {
            tipo_documento,
            documento,
            nombre,
            apellido,
            fecha_nacimiento: self.fecha_nacimiento,
            telefono,
            cargo_id: self.cargo_id,
            fecha_ingreso: self.fecha_ingreso,
        })
    }
}

impl Validar for PersonalUpdatePayload {
    type Datos = PersonalActualizado;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // La tabla arranca en 1 y solo sube; un valor menor no salió de un read.
        if self.version < 1 {
            return Err(ValidacionError::nuevo(
                "la version del empleado no es válida, recarga los datos",
            ));
        }

        let empleado_id = self.empleado_id;
        let version = self.version;

        // Las reglas de los campos son idénticas a las del alta, así que se
        // reusan en vez de repetirse: si cambia un largo o un tipo de documento,
        // cambia en un solo lugar.
        let datos = PersonalAddPayload {
            tipo_documento: self.tipo_documento,
            documento: self.documento,
            nombre: self.nombre,
            apellido: self.apellido,
            fecha_nacimiento: self.fecha_nacimiento,
            telefono: self.telefono,
            cargo_id: self.cargo_id,
            fecha_ingreso: self.fecha_ingreso,
        }
        .validar()?;

        Ok(PersonalActualizado {
            empleado_id,
            version,
            datos,
        })
    }
}
