use sqlx::PgPool;

use crate::{models::catalogos::materias_primas::MateriasPrimas, services::error::ServiceError};

/// Catálogo completo, activas e inactivas: son pocas filas y el cliente las
/// necesita todas para poder mostrar el nombre de una materia prima que ya no
/// se recibe pero que sigue referenciada en registros viejos.
pub async fn get_materias_primas(pool: &PgPool) -> Result<Vec<MateriasPrimas>, ServiceError> {
    // Los tres porcentajes se castean a float8 en el SELECT: la columna es
    // NUMERIC y el modelo los recibe como f64.
    let materias_primas = sqlx::query_as!(
        MateriasPrimas,
        r#"
        SELECT id, codigo, nombre, horas_maximas_espera,
               rendimiento_esperado_pct::float8 AS "rendimiento_esperado_pct",
               rendimiento_min_pct::float8      AS "rendimiento_min_pct",
               rendimiento_max_pct::float8      AS "rendimiento_max_pct",
               activo, version, creado_en, actualizado_en
        FROM materias_primas
        ORDER BY nombre
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(materias_primas)
}
