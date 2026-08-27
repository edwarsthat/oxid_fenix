use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, FromRow, Serialize)]
pub struct MateriasPrimas {
    pub id: Uuid,

    pub codigo: String,
    pub nombre: String,

    pub horas_maximas_espera: Option<i32>,

    // Los porcentajes son NUMERIC(5,2) en la tabla y viajan como f64: el
    // driver no trae tipo decimal habilitado y con dos decimales acotados a
    // 0..100 no hay pérdida que importe. Si algún día se hace aritmética de
    // dinero con esto, hay que subirlo a un decimal de verdad.
    pub rendimiento_esperado_pct: Option<f64>,

    pub rendimiento_min_pct: Option<f64>,
    pub rendimiento_max_pct: Option<f64>,

    pub activo: bool,
    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}
