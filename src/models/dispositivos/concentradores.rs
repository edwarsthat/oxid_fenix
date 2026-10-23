use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Una Raspberry Pi autorizada a mandar pesajes. Las filas se cargan a mano en
/// la base: no hay pantalla de alta, así que tampoco hay payloads ni validar().
///
/// No deriva `Serialize` a propósito: el único consumidor es el middleware que
/// autentica la petición, y así `llave_hash` no puede terminar en una
/// respuesta por accidente.
#[derive(Debug, FromRow)]
pub struct Concentrador {
    pub id: Uuid,
    pub codigo: String,

    pub nombre: String,
    pub ubicacion: Option<String>,
    pub descripcion: Option<String>,

    /// SHA-256 en hex de la API key. La Pi no manda ningún id: esto es lo que
    /// la identifica en cada petición.
    pub llave_hash: String,
    pub llave_prefijo: String,
    pub llave_emitida_en: DateTime<Utc>,

    pub estado: String,

    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}
