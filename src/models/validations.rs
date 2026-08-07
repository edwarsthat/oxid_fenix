use serde::Deserialize;

pub const LIMITE_POR_DEFECTO: i64 = 500;
pub const LIMITE_MAXIMO: i64 = 1000;

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
