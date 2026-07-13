use sqlx::PgPool;

use crate::{models::cargo::Cargo, services::error::ServiceError};


pub async fn get_cargos(
    pool: &PgPool,
) -> Result<Vec<Cargo>, ServiceError> {
    let cargos = sqlx::query_as!(
        Cargo,
        "SELECT id, nombre, descripcion, creado_en FROM cargos"
    )
    .fetch_all(pool)
    .await?;

    Ok(cargos)
}