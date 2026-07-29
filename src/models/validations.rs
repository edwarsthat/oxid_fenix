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
