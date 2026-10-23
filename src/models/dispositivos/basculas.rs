use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

/// Un ESP32 pegado a una báscula de la línea. Igual que los concentradores,
/// las filas se cargan a mano en la base: no hay payloads ni validar().
///
/// Al endpoint de pesajes le sirve para dos cosas: confirmar que el
/// `device_id` que trae el pesaje es una báscula conocida de ESE concentrador,
/// y saber en qué `linea` está, que es de donde sale el lote.
#[derive(Debug, FromRow)]
pub struct Bascula {
    pub id: Uuid,
    pub codigo: String,

    /// La MAC del ESP32, en minúsculas y con ':'. El CHECK de la tabla ya lo
    /// garantiza; lo que llega en el pesaje hay que pasarlo a minúsculas antes
    /// de buscar.
    pub device_id: String,
    pub concentrador_id: Uuid,

    pub linea: i16,

    pub nombre: String,
    pub ubicacion: Option<String>,
    pub descripcion: Option<String>,

    pub estado: String,

    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}
