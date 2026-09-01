use serde::Deserialize;
use uuid::Uuid;

pub const LIMITE_POR_DEFECTO: i64 = 500;
pub const LIMITE_MAXIMO: i64 = 1000;

/// Tope de un NUMERIC(12,2), que es el tipo de todas las columnas de peso:
/// diez dígitos enteros. Un número mayor no cabe y Postgres lo devolvería como
/// 22003 (numeric field overflow), o sea un error 500 por un dato del cliente.
pub const PESO_MAXIMO: f64 = 9_999_999_999.99;

/// El valor más chico que sobrevive al redondeo a dos decimales. Cualquier cosa
/// por debajo se guarda como 0.00, y ahí revienta el CHECK `> 0` de la columna
/// con un 23514 que no dice nada útil.
pub const PESO_MINIMO: f64 = 0.01;

/// Error de validación de una entrada. Siempre es culpa del cliente → 400.
/// No sabe nada de HTTP ni de WebSocket: cada transporte lo adapta.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ValidacionError(String);

impl ValidacionError {
    pub fn nuevo(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }

    pub fn mensaje(&self) -> &str {
        &self.0
    }
}

pub trait Validar {
    type Datos;

    fn validar(self) -> Result<Self::Datos, ValidacionError>;
}

#[derive(Debug, Deserialize)]
pub struct Rango<T> {
    pub desde: Option<T>,
    pub hasta: Option<T>,
}

#[derive(Debug, Clone, Copy)]
pub struct RangoValidado<T> {
    desde: Option<T>,
    hasta: Option<T>,
}

impl<T: Copy> RangoValidado<T> {
    pub fn desde(&self) -> Option<T> {
        self.desde
    }
    pub fn hasta(&self) -> Option<T> {
        self.hasta
    }
    pub fn vacio(&self) -> bool {
        self.desde.is_none() && self.hasta.is_none()
    }
}

pub fn limpiar_busqueda(texto: &str) -> Option<String> {
    let texto = texto.trim();

    if texto.is_empty() {
        return None;
    }

    Some(
        texto
            .replace('\\', "\\\\")
            .replace('%', "\\%")
            .replace('_', "\\_"),
    )
}

/// Texto que la columna exige NOT NULL: recorta, rechaza vacíos y corta antes
/// de que Postgres devuelva un 22001 (value too long) como error 500.
pub fn texto_obligatorio(
    valor: &str,
    campo: &str,
    maximo: usize,
) -> Result<String, ValidacionError> {
    let valor = valor.trim();

    if valor.is_empty() {
        return Err(ValidacionError::nuevo(format!(
            "el campo {campo} no puede estar vacío"
        )));
    }

    // VARCHAR(n) en Postgres cuenta caracteres, no bytes.
    if valor.chars().count() > maximo {
        return Err(ValidacionError::nuevo(format!(
            "el campo {campo} no puede superar {maximo} caracteres"
        )));
    }

    Ok(valor.to_string())
}

/// Igual que `texto_obligatorio`, pero un texto en blanco se guarda como NULL
/// en vez de como cadena vacía.
pub fn texto_opcional(
    valor: Option<&str>,
    campo: &str,
    maximo: usize,
) -> Result<Option<String>, ValidacionError> {
    match valor.map(str::trim) {
        None | Some("") => Ok(None),
        Some(valor) => Ok(Some(texto_obligatorio(valor, campo, maximo)?)),
    }
}

/// Los ids llegan como texto (el cliente manda JSON), así que el parseo y su
/// error son los mismos para todos: se resuelven en un solo lugar.
pub fn uuid_obligatorio(valor: &str, campo: &str) -> Result<Uuid, ValidacionError> {
    Uuid::parse_str(valor.trim())
        .map_err(|_| ValidacionError::nuevo(format!("el {campo} no es un UUID válido")))
}

/// Igual que `uuid_obligatorio`, pero la ausencia del id no es un error.
pub fn uuid_opcional(valor: Option<String>, campo: &str) -> Result<Option<Uuid>, ValidacionError> {
    valor
        .map(|texto| uuid_obligatorio(&texto, campo))
        .transpose()
}

/// Para el id que viaja como campo suelto del payload: si el cliente lo omite,
/// el error lo damos nosotros en español en vez del "missing field" de serde.
pub fn uuid_requerido(valor: Option<String>, campo: &str) -> Result<Uuid, ValidacionError> {
    let valor = valor.ok_or_else(|| ValidacionError::nuevo(format!("falta el {campo}")))?;

    uuid_obligatorio(&valor, campo)
}

/// Cantidad para una columna NUMERIC(12,2): finita, no negativa y dentro de lo
/// que aguanta la columna. Admite el cero, para los campos que arrancan en 0
/// (`peso_devuelto` y compañía).
///
/// Validar el rango acá y no dejárselo al INSERT es lo que convierte un 500 del
/// driver en un 400 que dice cuál campo está mal.
pub fn peso(valor: f64, campo: &str) -> Result<f64, ValidacionError> {
    // NaN e infinito no entran por los `<`/`>` de abajo: NaN no es mayor ni
    // menor que nada, así que se descarta primero y explícitamente.
    if !valor.is_finite() {
        return Err(ValidacionError::nuevo(format!(
            "el {campo} debe ser un número"
        )));
    }

    if valor < 0.0 {
        return Err(ValidacionError::nuevo(format!(
            "el {campo} no puede ser negativo"
        )));
    }

    if valor > PESO_MAXIMO {
        return Err(ValidacionError::nuevo(format!(
            "el {campo} supera el máximo que admite la columna"
        )));
    }

    Ok(valor)
}

/// Igual que `peso`, pero para las columnas con CHECK `> 0`. El mínimo no es
/// cero sino `PESO_MINIMO`: un 0.004 pasaría el `> 0` de Rust y se guardaría
/// como 0.00, reventando el CHECK ya dentro del INSERT.
pub fn peso_positivo(valor: f64, campo: &str) -> Result<f64, ValidacionError> {
    let valor = peso(valor, campo)?;

    if valor < PESO_MINIMO {
        return Err(ValidacionError::nuevo(format!(
            "el {campo} debe ser mayor que cero"
        )));
    }

    Ok(valor)
}

/// Igual que `peso`, pero para una columna que admite NULL.
pub fn peso_opcional(valor: Option<f64>, campo: &str) -> Result<Option<f64>, ValidacionError> {
    valor.map(|valor| peso(valor, campo)).transpose()
}

pub fn limitar(limite: Option<i64>) -> i64 {
    limite.unwrap_or(LIMITE_POR_DEFECTO).clamp(1, LIMITE_MAXIMO)
}

impl<T: PartialOrd> Validar for Rango<T> {
    type Datos = RangoValidado<T>;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        if let (Some(desde), Some(hasta)) = (&self.desde, &self.hasta)
            && desde > hasta
        {
            return Err(ValidacionError::nuevo(
                "la fecha inicial no puede ser posterior a la final",
            ));
        }

        Ok(RangoValidado {
            desde: self.desde,
            hasta: self.hasta,
        })
    }
}
