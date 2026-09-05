use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize)]
pub struct InventarioMateriaPrima {
    pub lote_id: Uuid,
    pub codigo: String,

    pub materia_prima_id: Uuid,
    pub materia_prima: String,
    pub predio: String,
    pub proveedor: String,

    pub llegada_en: DateTime<Utc>,

    pub peso_ingreso: f64,
    pub consumido: f64,
    pub saldo: f64,
}
