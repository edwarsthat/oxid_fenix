use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{
    LIMITE_MAXIMO, ValidacionError, Validar, limpiar_busqueda, texto_obligatorio, texto_opcional,
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
