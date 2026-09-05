use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize)]
pub struct MovimientosMateriaPrima {
    pub id: Uuid,
    pub clave_idempotencia: Uuid,

    pub lote_id: Uuid,
    pub programcion_id: Uuid,

    pub tipo: String,
    pub peso: f64,
    pub peso_efecto: f64,

    pub operario_id: Uuid,
    pub registrado_por: Uuid,

    pub motivo: String,
    pub observacion: String,

    pub corrige_movimiento_id: Uuid,

    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub acctualizado_en: DateTime<Utc>,
}
