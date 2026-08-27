use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{
    ValidacionError, Validar, texto_obligatorio, texto_opcional, uuid_requerido,
};

#[derive(Debug, FromRow, Serialize)]
pub struct Predios {
    pub id: Uuid,
    pub codigo: String,
    pub proveedor_id: Uuid,
    pub nombre: String,

    pub departamento: String,
    pub municipio: String,
    pub vereda: Option<String>,
    pub referencia_ubicacion: Option<String>,

    // La coordenada es NUMERIC(9,6) en la tabla y viaja como f64, igual que
    // los porcentajes de materias primas: el driver no trae tipo decimal
    // habilitado. El SELECT del servicio tiene que castearlas a float8.
    pub latitud: Option<f64>,
    pub longitud: Option<f64>,

    pub responsable_nombre: Option<String>,
    pub responsable_documento: Option<String>,
    pub responsable_telefono: Option<String>,

    pub observaciones: Option<String>,
    pub activo: bool,
    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct PredioAddPayload {
    pub proveedor_id: Option<String>,
    pub nombre: String,
    pub departamento: String,
    pub municipio: String,
    pub vereda: Option<String>,
    pub referencia_ubicacion: Option<String>,
    pub latitud: Option<String>,
    pub longitud: Option<String>,
    pub responsable_nombre: Option<String>,
    pub responsable_documento: Option<String>,
    pub responsable_telefono: Option<String>,
    pub observaciones: Option<String>,
}

/// Lo que devuelve validar() y consume el servicio: si tenés un `PredioNuevo`,
/// los datos ya están normalizados y son válidos.
#[derive(Debug)]
pub struct PredioNuevo {
    pub proveedor_id: Uuid,
    pub nombre: String,
    pub departamento: String,
    pub municipio: String,
    pub vereda: Option<String>,
    pub referencia_ubicacion: Option<String>,
    pub latitud: Option<f64>,
    pub longitud: Option<f64>,
    pub responsable_nombre: Option<String>,
    pub responsable_documento: Option<String>,
    pub responsable_telefono: Option<String>,
    pub observaciones: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PrediosReadPayload {
    pub activo: Option<bool>,
    pub proveedor_id: Option<Uuid>,
    pub busqueda: Option<String>,
    pub departamento: Option<String>,
}

#[derive(Debug)]
pub struct PrediosFiltros {
    pub activo: Option<bool>,
    pub proveedor_id: Option<Uuid>,
    pub busqueda: Option<String>,
    pub departamento: Option<String>,
    pub limite: i64,
}
/// Parsea una coordenada que llega como texto y la acota a su rango. El CHECK
/// de la tabla la atraparía igual, pero como 23514 genérico: validarla acá deja
/// el mensaje diciendo cuál de las dos está mal y entre qué valores va.
fn coordenada(
    valor: Option<&str>,
    campo: &str,
    maximo: f64,
) -> Result<Option<f64>, ValidacionError> {
    match valor.map(str::trim) {
        None | Some("") => Ok(None),
        Some(texto) => {
            let numero: f64 = texto.parse().map_err(|_| {
                ValidacionError::nuevo(format!("la {campo} debe ser un número en grados decimales"))
            })?;

            // Un NaN no entra por acá: no está contenido en ningún rango.
            if !(-maximo..=maximo).contains(&numero) {
                return Err(ValidacionError::nuevo(format!(
                    "la {campo} debe estar entre -{maximo} y {maximo}"
                )));
            }

            Ok(Some(numero))
        }
    }
}

impl Validar for PredioAddPayload {
    type Datos = PredioNuevo;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let proveedor_id = uuid_requerido(self.proveedor_id, "proveedor_id")?;

        // Los largos son los de la migración: se cortan acá para que Postgres
        // no devuelva un 22001 (value too long) como error 500.
        let nombre = texto_obligatorio(&self.nombre, "nombre", 200)?;
        let departamento = texto_obligatorio(&self.departamento, "departamento", 80)?;
        let municipio = texto_obligatorio(&self.municipio, "municipio", 80)?;

        let vereda = texto_opcional(self.vereda.as_deref(), "vereda", 120)?;
        let referencia_ubicacion = texto_opcional(
            self.referencia_ubicacion.as_deref(),
            "referencia_ubicacion",
            300,
        )?;

        let latitud = coordenada(self.latitud.as_deref(), "latitud", 90.0)?;
        let longitud = coordenada(self.longitud.as_deref(), "longitud", 180.0)?;

        // Misma regla que el CHECK `predios_coordenadas_completas_check`: media
        // coordenada no ubica nada, así que o van las dos o no va ninguna.
        if latitud.is_some() != longitud.is_some() {
            return Err(ValidacionError::nuevo(
                "la latitud y la longitud van juntas: falta una de las dos",
            ));
        }

        let responsable_nombre = texto_opcional(
            self.responsable_nombre.as_deref(),
            "responsable_nombre",
            150,
        )?;
        let responsable_documento = texto_opcional(
            self.responsable_documento.as_deref(),
            "responsable_documento",
            30,
        )?;
        let responsable_telefono = texto_opcional(
            self.responsable_telefono.as_deref(),
            "responsable_telefono",
            30,
        )?;

        let observaciones = texto_opcional(self.observaciones.as_deref(), "observaciones", 500)?;

        Ok(PredioNuevo {
            proveedor_id,
            nombre,
            departamento,
            municipio,
            vereda,
            referencia_ubicacion,
            latitud,
            longitud,
            responsable_nombre,
            responsable_documento,
            responsable_telefono,
            observaciones,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_payload() -> PredioAddPayload {
        PredioAddPayload {
            proveedor_id: Some(Uuid::new_v4().to_string()),
            nombre: "La Esperanza".into(),
            departamento: "Valle del Cauca".into(),
            municipio: "Palmira".into(),
            vereda: None,
            referencia_ubicacion: None,
            latitud: None,
            longitud: None,
            responsable_nombre: None,
            responsable_documento: None,
            responsable_telefono: None,
            observaciones: None,
        }
    }

    #[test]
    fn validar_normaliza_y_devuelve_los_datos() {
        let id = Uuid::new_v4();
        let payload = PredioAddPayload {
            proveedor_id: Some(format!("  {id}  ")),
            nombre: "  La Esperanza  ".into(),
            latitud: Some(" 3.539444 ".into()),
            longitud: Some("-76.303889".into()),
            ..add_payload()
        };

        let nuevo = payload.validar().expect("deberia ser valido");

        assert_eq!(nuevo.proveedor_id, id);
        assert_eq!(nuevo.nombre, "La Esperanza");
        assert_eq!(nuevo.latitud, Some(3.539444));
        assert_eq!(nuevo.longitud, Some(-76.303889));
    }

    /// Un opcional en blanco se guarda como NULL, no como cadena vacía.
    #[test]
    fn los_opcionales_en_blanco_quedan_en_none() {
        let payload = PredioAddPayload {
            vereda: Some("   ".into()),
            observaciones: Some("".into()),
            latitud: Some("  ".into()),
            longitud: None,
            ..add_payload()
        };

        let nuevo = payload.validar().expect("deberia ser valido");

        assert_eq!(nuevo.vereda, None);
        assert_eq!(nuevo.observaciones, None);
        assert_eq!(nuevo.latitud, None);
    }

    #[test]
    fn validar_rechaza_los_obligatorios_vacios() {
        let payload = PredioAddPayload {
            nombre: "   ".into(),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el campo nombre no puede estar vacío"
        );

        let payload = PredioAddPayload {
            municipio: "".into(),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el campo municipio no puede estar vacío"
        );
    }

    #[test]
    fn validar_rechaza_proveedor_id_ausente_o_invalido() {
        let payload = PredioAddPayload {
            proveedor_id: None,
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "falta el proveedor_id"
        );

        let payload = PredioAddPayload {
            proveedor_id: Some("no-es-un-uuid".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el proveedor_id no es un UUID válido"
        );
    }

    /// Las mismas reglas que los CHECK de la tabla, pero con un mensaje que
    /// dice qué corregir en vez de un 23514.
    #[test]
    fn validar_rechaza_media_coordenada() {
        let payload = PredioAddPayload {
            latitud: Some("3.539444".into()),
            longitud: None,
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la latitud y la longitud van juntas: falta una de las dos"
        );
    }

    #[test]
    fn validar_rechaza_coordenadas_fuera_de_rango() {
        let payload = PredioAddPayload {
            latitud: Some("200".into()),
            longitud: Some("-76.3".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la latitud debe estar entre -90 y 90"
        );

        let payload = PredioAddPayload {
            latitud: Some("3.5".into()),
            longitud: Some("-500".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la longitud debe estar entre -180 y 180"
        );
    }

    #[test]
    fn validar_rechaza_una_coordenada_que_no_es_numero() {
        let payload = PredioAddPayload {
            latitud: Some("norte".into()),
            longitud: Some("-76.3".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la latitud debe ser un número en grados decimales"
        );
    }

    /// El texto se corta acá y no en el INSERT: la columna es VARCHAR(200).
    #[test]
    fn validar_rechaza_un_nombre_demasiado_largo() {
        let payload = PredioAddPayload {
            nombre: "a".repeat(201),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el campo nombre no puede superar 200 caracteres"
        );
    }
}
