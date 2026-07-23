use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, FromRow, Serialize)]
pub struct Usuario {
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub usuario: String,
    pub password_hash: String,
    pub cargo_id: Uuid,
    pub activo: bool,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}

#[derive(Debug, FromRow, Serialize)]
pub struct UsuarioListItem {
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub usuario: String,
    pub cargo_id: Uuid,
    pub activo: bool,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}