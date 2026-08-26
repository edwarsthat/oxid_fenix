use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{
    LIMITE_MAXIMO, ValidacionError, Validar, limpiar_busqueda, texto_obligatorio, texto_opcional,
    uuid_requerido,
};

#[derive(Debug, FromRow, Serialize)]
pub struct Proveedor {
    pub id: Uuid,
    pub codigo: String,
    pub tipo_proveedor: String,

    pub tipo_persona: String,
    pub tipo_documento: String,
    pub documento: String,
    pub digito_verificacion: Option<String>,

    pub nombre: String,
    pub razon_social: Option<String>,

    pub telefono: Option<String>,
    pub telefono_alterno: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub departamento: Option<String>,
    pub municipio: Option<String>,
    pub contacto_nombre: Option<String>,
    pub contacto_telefono: Option<String>,

    pub banco: Option<String>,
    pub tipo_cuenta: Option<String>,
    pub numero_cuenta: Option<String>,
    pub titular_cuenta: Option<String>,
    pub titular_documento: Option<String>,

    pub observaciones: Option<String>,
    pub activo: bool,
    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}

/// Tipos de proveedor aceptados por la columna `tipo_proveedor`. Es la misma
/// lista del CHECK de la migración: si cambia una, cambia la otra.
const TIPOS_PROVEEDOR: [&str; 3] = ["materia_prima", "insumo", "servicio"];

/// Lo mismo para `tipo_persona`.
const TIPOS_PERSONA: [&str; 2] = ["natural", "juridica"];

/// Y para `tipo_cuenta`, que solo aplica cuando se cargan datos bancarios.
const TIPOS_CUENTA: [&str; 2] = ["ahorros", "corriente"];

/// Tipos de documento aceptados por la columna `tipo_documento`, igual que en
/// personal: el proveedor puede ser la misma persona natural que un empleado.
const TIPOS_DOCUMENTO: [&str; 8] = ["CC", "CE", "TI", "RC", "PA", "NIT", "PEP", "PPT"];

#[derive(Debug, Deserialize)]
pub struct ProveedorAddPayload {
    pub tipo_proveedor: String,
    pub tipo_persona: Option<String>,
    pub tipo_documento: Option<String>,
    pub documento: String,
    pub digito_verificacion: Option<String>,

    pub nombre: String,
    pub razon_social: Option<String>,

    pub telefono: Option<String>,
    pub telefono_alterno: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub departamento: Option<String>,
    pub municipio: Option<String>,
    pub contacto_nombre: Option<String>,
    pub contacto_telefono: Option<String>,

    pub banco: Option<String>,
    pub tipo_cuenta: Option<String>,
    pub numero_cuenta: Option<String>,
    pub titular_cuenta: Option<String>,
    pub titular_documento: Option<String>,

    pub observaciones: Option<String>,
}

/// Lo que devuelve validar() y consume el servicio: si tenés un
/// `ProveedorNuevo`, los datos ya están normalizados y son válidos.
#[derive(Debug)]
pub struct ProveedorNuevo {
    pub tipo_proveedor: String,
    pub tipo_persona: String,
    pub tipo_documento: String,
    pub documento: String,
    pub digito_verificacion: Option<String>,

    pub nombre: String,
    pub razon_social: Option<String>,

    pub telefono: Option<String>,
    pub telefono_alterno: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub departamento: Option<String>,
    pub municipio: Option<String>,
    pub contacto_nombre: Option<String>,
    pub contacto_telefono: Option<String>,

    pub banco: Option<String>,
    pub tipo_cuenta: Option<String>,
    pub numero_cuenta: Option<String>,
    pub titular_cuenta: Option<String>,
    pub titular_documento: Option<String>,

    pub observaciones: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProveedorReadPayload {
    pub activo: Option<bool>,
    pub busqueda: Option<String>,
    pub tipo_proveedor: Option<String>,
    pub tipo_persona: Option<String>,
    pub departamento: Option<String>,
}

#[derive(Debug)]
pub struct ProveedoresFiltros {
    pub activo: bool,
    pub busqueda: Option<String>,
    pub tipo_proveedor: Option<String>,
    pub tipo_persona: Option<String>,
    pub departamento: Option<String>,
    pub limite: i64,
}

/// Lo que manda el cliente para editar. Es un reemplazo completo: viajan todos
/// los campos, no solo los que cambiaron, así que las reglas son las mismas del
/// alta más el id y la version.
#[derive(Debug, Deserialize)]
pub struct ProveedorUpdatePayload {
    pub proveedor_id: Option<String>,

    pub tipo_proveedor: String,
    pub tipo_persona: Option<String>,
    pub tipo_documento: Option<String>,
    pub documento: String,
    pub digito_verificacion: Option<String>,

    pub nombre: String,
    pub razon_social: Option<String>,

    pub telefono: Option<String>,
    pub telefono_alterno: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub departamento: Option<String>,
    pub municipio: Option<String>,
    pub contacto_nombre: Option<String>,
    pub contacto_telefono: Option<String>,

    pub banco: Option<String>,
    pub tipo_cuenta: Option<String>,
    pub numero_cuenta: Option<String>,
    pub titular_cuenta: Option<String>,
    pub titular_documento: Option<String>,

    pub observaciones: Option<String>,

    pub version: i32,
}

/// Lo que devuelve validar() y consume el servicio. Los campos editables no se
/// repiten acá: son los mismos del alta y viajan dentro de `datos`, así el
/// UPDATE y el INSERT no pueden quedar validando reglas distintas.
#[derive(Debug)]
pub struct ProveedorActualizado {
    pub proveedor_id: Uuid,
    pub version: i32,
    pub datos: ProveedorNuevo,
}

/// Payload de las operaciones que solo señalan a un proveedor (dar de baja).
#[derive(Debug, Deserialize)]
pub struct ProveedorIdPayload {
    pub proveedor_id: Option<String>,
}

impl Validar for ProveedorIdPayload {
    type Datos = Uuid;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        uuid_requerido(self.proveedor_id, "proveedor_id")
    }
}

/// Normaliza y comprueba un valor contra la misma lista del CHECK. En el INSERT
/// la tabla igual lo atraparía, pero como 23514 genérico: validarlo acá deja el
/// mensaje con los valores que sí sirven.
fn opcion_de_lista(valor: &str, campo: &str, opciones: &[&str]) -> Result<String, ValidacionError> {
    let valor = valor.trim().to_lowercase();

    if !opciones.contains(&valor.as_str()) {
        return Err(ValidacionError::nuevo(format!(
            "el {campo} debe ser uno de: {}",
            opciones.join(", ")
        )));
    }

    Ok(valor)
}

/// Chequeo mínimo: un `@`, algo a cada lado y un punto en el dominio. No intenta
/// ser un validador de RFC 5322, solo atajar dedazos.
fn es_email_valido(email: &str) -> bool {
    let Some((local, dominio)) = email.split_once('@') else {
        return false;
    };

    !local.is_empty()
        && !dominio.is_empty()
        && !dominio.contains('@')
        && dominio.contains('.')
        && !dominio.starts_with('.')
        && !dominio.ends_with('.')
        && !email.contains(char::is_whitespace)
}

impl Validar for ProveedorAddPayload {
    type Datos = ProveedorNuevo;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let tipo_proveedor =
            opcion_de_lista(&self.tipo_proveedor, "tipo_proveedor", &TIPOS_PROVEEDOR)?;

        // Los dos campos con DEFAULT en la tabla se pueden omitir: si el
        // cliente no los manda, acá se aplica el mismo default de la migración
        // para que el servicio siempre reciba el valor completo.
        let tipo_persona = match self.tipo_persona.as_deref().map(str::trim) {
            None | Some("") => "natural".to_string(),
            Some(valor) => opcion_de_lista(valor, "tipo_persona", &TIPOS_PERSONA)?,
        };

        let tipo_documento = match self.tipo_documento.as_deref().map(str::trim) {
            None | Some("") => "CC".to_string(),
            Some(valor) => {
                let valor = valor.to_uppercase();

                if !TIPOS_DOCUMENTO.contains(&valor.as_str()) {
                    return Err(ValidacionError::nuevo(format!(
                        "el tipo_documento debe ser uno de: {}",
                        TIPOS_DOCUMENTO.join(", ")
                    )));
                }

                valor
            }
        };

        let documento = texto_obligatorio(&self.documento, "documento", 30)?;
        let nombre = texto_obligatorio(&self.nombre, "nombre", 200)?;
        let razon_social = texto_opcional(self.razon_social.as_deref(), "razon_social", 200)?;

        // Una empresa se factura a nombre de su razón social; sin ella el
        // registro no sirve para lo único que se usa un proveedor jurídico.
        if tipo_persona == "juridica" && razon_social.is_none() {
            return Err(ValidacionError::nuevo(
                "una persona juridica debe tener razon_social",
            ));
        }

        // El dígito de verificación es parte del NIT, no un campo suelto: con
        // otro tipo de documento no hay nada que verificar.
        let digito_verificacion = match self.digito_verificacion.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(_) if tipo_documento != "NIT" => {
                return Err(ValidacionError::nuevo(
                    "el digito_verificacion solo aplica a un NIT",
                ));
            }
            Some(valor) => {
                // La columna es CHAR(1) y el DV del NIT va de 0 a 9.
                if valor.len() != 1 || !valor.chars().all(|c| c.is_ascii_digit()) {
                    return Err(ValidacionError::nuevo(
                        "el digito_verificacion debe ser un digito del 0 al 9",
                    ));
                }

                Some(valor.to_string())
            }
        };

        let telefono = texto_opcional(self.telefono.as_deref(), "telefono", 30)?;
        let telefono_alterno =
            texto_opcional(self.telefono_alterno.as_deref(), "telefono_alterno", 30)?;

        let email = match texto_opcional(self.email.as_deref(), "email", 150)? {
            None => None,
            Some(valor) => {
                // En minúsculas para que el mismo correo escrito de dos formas
                // no quede como dos proveedores distintos.
                let valor = valor.to_lowercase();

                if !es_email_valido(&valor) {
                    return Err(ValidacionError::nuevo("el email no es valido"));
                }

                Some(valor)
            }
        };

        let direccion = texto_opcional(self.direccion.as_deref(), "direccion", 200)?;
        let departamento = texto_opcional(self.departamento.as_deref(), "departamento", 80)?;
        let municipio = texto_opcional(self.municipio.as_deref(), "municipio", 80)?;
        let contacto_nombre =
            texto_opcional(self.contacto_nombre.as_deref(), "contacto_nombre", 150)?;
        let contacto_telefono =
            texto_opcional(self.contacto_telefono.as_deref(), "contacto_telefono", 30)?;

        let banco = texto_opcional(self.banco.as_deref(), "banco", 80)?;

        let tipo_cuenta = match self.tipo_cuenta.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(valor) => Some(opcion_de_lista(valor, "tipo_cuenta", &TIPOS_CUENTA)?),
        };

        let numero_cuenta = texto_opcional(self.numero_cuenta.as_deref(), "numero_cuenta", 30)?;
        let titular_cuenta = texto_opcional(self.titular_cuenta.as_deref(), "titular_cuenta", 200)?;
        let titular_documento =
            texto_opcional(self.titular_documento.as_deref(), "titular_documento", 30)?;

        // Los datos bancarios van juntos o no van: un número de cuenta sin
        // banco ni tipo no alcanza para pagarle a nadie, y la tabla los deja
        // pasar sueltos porque los tres son NULL-ables.
        if numero_cuenta.is_some() && (banco.is_none() || tipo_cuenta.is_none()) {
            return Err(ValidacionError::nuevo(
                "si se carga un numero_cuenta hay que indicar banco y tipo_cuenta",
            ));
        }

        let observaciones = texto_opcional(self.observaciones.as_deref(), "observaciones", 500)?;

        Ok(ProveedorNuevo {
            tipo_proveedor,
            tipo_persona,
            tipo_documento,
            documento,
            digito_verificacion,
            nombre,
            razon_social,
            telefono,
            telefono_alterno,
            email,
            direccion,
            departamento,
            municipio,
            contacto_nombre,
            contacto_telefono,
            banco,
            tipo_cuenta,
            numero_cuenta,
            titular_cuenta,
            titular_documento,
            observaciones,
        })
    }
}

impl Validar for ProveedorReadPayload {
    type Datos = ProveedoresFiltros;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // Por defecto el listado trae los proveedores vigentes: los dados de
        // baja se piden a propósito, no se cuelan en la vista de todos los días.
        let activo = self.activo.unwrap_or(true);

        // Filtro de texto, no columna a guardar: va por `limpiar_busqueda` para
        // que un '%' escrito por el usuario no se lea como comodín del LIKE.
        let busqueda = match self.busqueda {
            Some(texto) => limpiar_busqueda(&texto),
            None => None,
        };

        // Un tipo en blanco es "sin filtro"; uno mal escrito es un error y no
        // un listado vacío: en un SELECT el CHECK de la tabla no atrapa nada.
        let tipo_proveedor = match self.tipo_proveedor.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(valor) => Some(opcion_de_lista(valor, "tipo_proveedor", &TIPOS_PROVEEDOR)?),
        };

        let tipo_persona = match self.tipo_persona.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(valor) => Some(opcion_de_lista(valor, "tipo_persona", &TIPOS_PERSONA)?),
        };

        // El departamento no tiene lista cerrada: se guarda como lo escribieron,
        // así que acá solo se recorta y se corta en el largo de la columna. El
        // servicio lo compara completo, sin comodines alrededor.
        let departamento = texto_opcional(self.departamento.as_deref(), "departamento", 80)?;

        let limite = LIMITE_MAXIMO;

        Ok(ProveedoresFiltros {
            activo,
            busqueda,
            tipo_proveedor,
            tipo_persona,
            departamento,
            limite,
        })
    }
}

impl Validar for ProveedorUpdatePayload {
    type Datos = ProveedorActualizado;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // La tabla arranca en 1 y solo sube; un valor menor no salió de un read.
        if self.version < 1 {
            return Err(ValidacionError::nuevo(
                "la version del proveedor no es válida, recarga los datos",
            ));
        }

        let proveedor_id = uuid_requerido(self.proveedor_id, "proveedor_id")?;
        let version = self.version;

        // Las reglas de los campos son idénticas a las del alta, así que se
        // reusan en vez de repetirse: si cambia un largo, una lista del CHECK o
        // la regla de los datos bancarios, cambia en un solo lugar.
        let datos = ProveedorAddPayload {
            tipo_proveedor: self.tipo_proveedor,
            tipo_persona: self.tipo_persona,
            tipo_documento: self.tipo_documento,
            documento: self.documento,
            digito_verificacion: self.digito_verificacion,
            nombre: self.nombre,
            razon_social: self.razon_social,
            telefono: self.telefono,
            telefono_alterno: self.telefono_alterno,
            email: self.email,
            direccion: self.direccion,
            departamento: self.departamento,
            municipio: self.municipio,
            contacto_nombre: self.contacto_nombre,
            contacto_telefono: self.contacto_telefono,
            banco: self.banco,
            tipo_cuenta: self.tipo_cuenta,
            numero_cuenta: self.numero_cuenta,
            titular_cuenta: self.titular_cuenta,
            titular_documento: self.titular_documento,
            observaciones: self.observaciones,
        }
        .validar()?;

        Ok(ProveedorActualizado {
            proveedor_id,
            version,
            datos,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update_payload() -> ProveedorUpdatePayload {
        ProveedorUpdatePayload {
            proveedor_id: Some(Uuid::new_v4().to_string()),
            tipo_proveedor: "insumo".into(),
            tipo_persona: None,
            tipo_documento: None,
            documento: "900123456".into(),
            digito_verificacion: None,
            nombre: "Insumos del Valle".into(),
            razon_social: None,
            telefono: None,
            telefono_alterno: None,
            email: None,
            direccion: None,
            departamento: None,
            municipio: None,
            contacto_nombre: None,
            contacto_telefono: None,
            banco: None,
            tipo_cuenta: None,
            numero_cuenta: None,
            titular_cuenta: None,
            titular_documento: None,
            observaciones: None,
            version: 1,
        }
    }

    #[test]
    fn validar_devuelve_id_version_y_datos_normalizados() {
        let id = Uuid::new_v4();
        let payload = ProveedorUpdatePayload {
            proveedor_id: Some(id.to_string()),
            tipo_proveedor: "  INSUMO  ".into(),
            nombre: "  Insumos del Valle  ".into(),
            version: 4,
            ..update_payload()
        };

        let actualizado = payload.validar().expect("deberia ser valido");

        assert_eq!(actualizado.proveedor_id, id);
        assert_eq!(actualizado.version, 4);
        assert_eq!(actualizado.datos.tipo_proveedor, "insumo");
        assert_eq!(actualizado.datos.nombre, "Insumos del Valle");
        // Los defaults de la tabla se aplican igual que en el alta.
        assert_eq!(actualizado.datos.tipo_persona, "natural");
        assert_eq!(actualizado.datos.tipo_documento, "CC");
    }

    #[test]
    fn validar_rechaza_version_menor_a_uno() {
        let payload = ProveedorUpdatePayload {
            version: 0,
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la version del proveedor no es válida, recarga los datos"
        );
    }

    #[test]
    fn validar_rechaza_proveedor_id_invalido() {
        let payload = ProveedorUpdatePayload {
            proveedor_id: Some("no-es-un-uuid".into()),
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el proveedor_id no es un UUID válido"
        );
    }

    #[test]
    fn validar_rechaza_proveedor_id_ausente() {
        let payload = ProveedorUpdatePayload {
            proveedor_id: None,
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "falta el proveedor_id"
        );
    }

    /// El update no revalida a mano: hereda las reglas del alta. Si esto se
    /// rompe, es que dejó de reusar `ProveedorAddPayload`.
    #[test]
    fn validar_hereda_las_reglas_del_alta() {
        let payload = ProveedorUpdatePayload {
            tipo_persona: Some("juridica".into()),
            razon_social: None,
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "una persona juridica debe tener razon_social"
        );

        let payload = ProveedorUpdatePayload {
            numero_cuenta: Some("123456".into()),
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "si se carga un numero_cuenta hay que indicar banco y tipo_cuenta"
        );
    }

    /// La version se valida antes que el resto: quien manda datos viejos tiene
    /// que recargar, no ponerse a corregir campos que igual va a perder.
    #[test]
    fn la_version_se_valida_antes_que_los_campos() {
        let payload = ProveedorUpdatePayload {
            version: 0,
            nombre: "   ".into(),
            ..update_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la version del proveedor no es válida, recarga los datos"
        );
    }

    #[test]
    fn validar_id_devuelve_el_uuid() {
        let id = Uuid::new_v4();
        let payload = ProveedorIdPayload {
            proveedor_id: Some(format!("  {id}  ")),
        };

        assert_eq!(payload.validar().expect("deberia ser valido"), id);
    }

    #[test]
    fn validar_id_rechaza_uuid_invalido() {
        let payload = ProveedorIdPayload {
            proveedor_id: Some("no-es-un-uuid".into()),
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el proveedor_id no es un UUID válido"
        );
    }

    #[test]
    fn validar_id_rechaza_ausencia() {
        let payload = ProveedorIdPayload { proveedor_id: None };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "falta el proveedor_id"
        );
    }
}
