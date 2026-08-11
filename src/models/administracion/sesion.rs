use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;

use crate::sessions::memory::Session;

/// Vista pública de una sesión viva.
///
/// No expone la clave del `SessionStore` (que es el token de sesión) ni el set
/// de permisos: lo primero permitiría suplantar al usuario, lo segundo solo
/// infla la respuesta. `Session` se queda sin `Serialize` a propósito, para que
/// este DTO sea el único camino hacia el cliente.
#[derive(Debug, Serialize)]
pub struct SesionInfo {
    pub usuario_id: Uuid,
    pub cargo_id: Uuid,
    pub expira_en: DateTime<Utc>,
}

impl From<&Session> for SesionInfo {
    fn from(s: &Session) -> Self {
        Self {
            usuario_id: s.usuario_id,
            cargo_id: s.cargo_id,
            expira_en: s.expira_en,
        }
    }
}
