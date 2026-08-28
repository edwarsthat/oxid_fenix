use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{
    LIMITE_MAXIMO, ValidacionError, Validar, limpiar_busqueda, texto_obligatorio, texto_opcional,
    uuid_obligatorio, uuid_opcional, uuid_requerido,
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

/// Lo que manda el cliente.
#[derive(Debug, Deserialize)]
pub struct PrediosReadPayload {
    pub activo: Option<bool>,
    pub proveedor_id: Option<String>,
    pub busqueda: Option<String>,
    pub departamento: Option<String>,
}

/// Lo que devuelve validar() y consume el servicio.
#[derive(Debug)]
pub struct PrediosFiltros {
    pub activo: bool,
    pub proveedor_id: Option<Uuid>,
    pub busqueda: Option<String>,
    pub departamento: Option<String>,
    pub limite: i64,
}

/// Lo que manda el cliente para editar. Es un reemplazo completo: viajan todos
/// los campos, no solo los que cambiaron, así que las reglas son las mismas del
/// alta más el id y la version.
#[derive(Debug, Deserialize)]
pub struct PredioUpdatePayload {
    pub predio_id: String,
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
    pub version: i32,
}

/// Lo que devuelve validar() y consume el servicio.
#[derive(Debug)]
pub struct PredioActualizado {
    pub predio_id: Uuid,
    pub version: i32,
    pub datos: PredioNuevo,
}

#[derive(Debug, Deserialize)]
pub struct PredioIdPayload {
    pub predio_id: Option<String>,
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

impl Validar for PrediosReadPayload {
    type Datos = PrediosFiltros;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // Por defecto el listado trae los predios vigentes: los dados de baja
        // se piden a propósito, no se cuelan en la vista de todos los días.
        let activo = self.activo.unwrap_or(true);

        // El id llega como texto igual que en el alta: un uuid mal escrito es
        // un error con mensaje propio y no un listado vacío.
        let proveedor_id = uuid_opcional(self.proveedor_id, "proveedor_id")?;

        // Filtro de texto, no columna a guardar: va por `limpiar_busqueda` para
        // que un '%' escrito por el usuario no se lea como comodín del LIKE.
        let busqueda = match self.busqueda {
            Some(texto) => limpiar_busqueda(&texto),
            None => None,
        };

        // El departamento no tiene lista cerrada: se guarda como lo escribieron,
        // así que acá solo se recorta y se corta en el largo de la columna. El
        // servicio lo compara completo, sin comodines alrededor.
        let departamento = texto_opcional(self.departamento.as_deref(), "departamento", 80)?;

        let limite = LIMITE_MAXIMO;

        Ok(PrediosFiltros {
            activo,
            proveedor_id,
            busqueda,
            departamento,
            limite,
        })
    }
}

impl Validar for PredioUpdatePayload {
    type Datos = PredioActualizado;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // La tabla arranca en 1 y solo sube; un valor menor no salió de un read.
        if self.version < 1 {
            return Err(ValidacionError::nuevo(
                "la version del predio no es válida, recarga los datos",
            ));
        }

        let predio_id = uuid_obligatorio(&self.predio_id, "predio_id")?;
        let version = self.version;

        // Las reglas de los campos son idénticas a las del alta, así que se
        // reusan en vez de repetirse: si cambia un largo o la regla de las
        // coordenadas, cambia en un solo lugar. El proveedor_id va incluido:
        // un predio se puede reasignar, pero nunca quedar sin proveedor.
        let datos = PredioAddPayload {
            proveedor_id: self.proveedor_id,
            nombre: self.nombre,
            departamento: self.departamento,
            municipio: self.municipio,
            vereda: self.vereda,
            referencia_ubicacion: self.referencia_ubicacion,
            latitud: self.latitud,
            longitud: self.longitud,
            responsable_nombre: self.responsable_nombre,
            responsable_documento: self.responsable_documento,
            responsable_telefono: self.responsable_telefono,
            observaciones: self.observaciones,
        }
        .validar()?;

        Ok(PredioActualizado {
            predio_id,
            version,
            datos,
        })
    }
}

impl Validar for PredioIdPayload {
    type Datos = Uuid;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        uuid_requerido(self.predio_id, "predio_id")
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

    fn read_payload() -> PrediosReadPayload {
        PrediosReadPayload {
            activo: None,
            proveedor_id: None,
            busqueda: None,
            departamento: None,
        }
    }

    /// Sin `activo` el listado trae los vigentes: los dados de baja se piden.
    #[test]
    fn los_filtros_traen_los_activos_por_defecto() {
        let filtros = read_payload().validar().expect("deberia ser valido");

        assert!(filtros.activo);
        assert_eq!(filtros.proveedor_id, None);
        assert_eq!(filtros.limite, LIMITE_MAXIMO);

        let payload = PrediosReadPayload {
            activo: Some(false),
            ..read_payload()
        };

        assert!(!payload.validar().expect("deberia ser valido").activo);
    }

    #[test]
    fn los_filtros_normalizan_proveedor_id_y_departamento() {
        let id = Uuid::new_v4();
        let payload = PrediosReadPayload {
            proveedor_id: Some(format!("  {id}  ")),
            departamento: Some("  Valle del Cauca  ".into()),
            ..read_payload()
        };

        let filtros = payload.validar().expect("deberia ser valido");

        assert_eq!(filtros.proveedor_id, Some(id));
        assert_eq!(filtros.departamento.as_deref(), Some("Valle del Cauca"));
    }

    /// Un filtro en blanco es "sin filtro", no un texto vacío que no matchea.
    #[test]
    fn los_filtros_en_blanco_quedan_en_none() {
        let payload = PrediosReadPayload {
            busqueda: Some("   ".into()),
            departamento: Some("".into()),
            ..read_payload()
        };

        let filtros = payload.validar().expect("deberia ser valido");

        assert_eq!(filtros.busqueda, None);
        assert_eq!(filtros.departamento, None);
    }

    /// El '%' que escriba el usuario se busca literal, no como comodín del LIKE.
    #[test]
    fn la_busqueda_escapa_los_comodines() {
        let payload = PrediosReadPayload {
            busqueda: Some(" 100% finca_1 ".into()),
            ..read_payload()
        };

        let filtros = payload.validar().expect("deberia ser valido");

        assert_eq!(filtros.busqueda.as_deref(), Some("100\\% finca\\_1"));
    }

    /// Un uuid mal escrito es un error con mensaje propio, no un listado vacío.
    #[test]
    fn los_filtros_rechazan_un_proveedor_id_invalido() {
        let payload = PrediosReadPayload {
            proveedor_id: Some("no-es-un-uuid".into()),
            ..read_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el proveedor_id no es un UUID válido"
        );
    }

    fn update_payload() -> PredioUpdatePayload {
        PredioUpdatePayload {
            predio_id: Uuid::new_v4().to_string(),
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
            version: 1,
        }
    }

    #[test]
    fn validar_devuelve_id_version_y_datos_normalizados() {
        let predio_id = Uuid::new_v4();
        let proveedor_id = Uuid::new_v4();
        let payload = PredioUpdatePayload {
            predio_id: format!("  {predio_id}  "),
            proveedor_id: Some(proveedor_id.to_string()),
            nombre: "  La Esperanza  ".into(),
            latitud: Some(" 3.539444 ".into()),
            longitud: Some("-76.303889".into()),
            version: 4,
            ..update_payload()
        };

        let actualizado = payload.validar().expect("deberia ser valido");

        assert_eq!(actualizado.predio_id, predio_id);
        assert_eq!(actualizado.version, 4);
        assert_eq!(actualizado.datos.proveedor_id, proveedor_id);
        assert_eq!(actualizado.datos.nombre, "La Esperanza");
        assert_eq!(actualizado.datos.latitud, Some(3.539444));
        assert_eq!(actualizado.datos.longitud, Some(-76.303889));
    }

    #[test]
    fn validar_rechaza_una_version_menor_a_uno() {
        let payload = PredioUpdatePayload {
            version: 0,
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la version del predio no es válida, recarga los datos"
        );
    }

    /// La version se valida antes que el resto: quien manda datos viejos tiene
    /// que recargar, no corregir campo por campo.
    #[test]
    fn la_version_se_valida_antes_que_los_campos() {
        let payload = PredioUpdatePayload {
            version: 0,
            nombre: "   ".into(),
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la version del predio no es válida, recarga los datos"
        );
    }

    #[test]
    fn validar_rechaza_un_predio_id_invalido() {
        let payload = PredioUpdatePayload {
            predio_id: "no-es-un-uuid".into(),
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el predio_id no es un UUID válido"
        );
    }

    /// El update reusa las reglas del alta: un predio no puede quedar sin
    /// proveedor ni con media coordenada.
    #[test]
    fn el_update_hereda_las_reglas_del_alta() {
        let payload = PredioUpdatePayload {
            proveedor_id: None,
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "falta el proveedor_id"
        );

        let payload = PredioUpdatePayload {
            latitud: Some("3.539444".into()),
            longitud: None,
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la latitud y la longitud van juntas: falta una de las dos"
        );
    }
}
