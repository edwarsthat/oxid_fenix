use sqlx::PgPool;

use crate::{models::permiso::PermisoResumen, services::error::ServiceError};


pub async fn get_permisos(
    pool: &PgPool
) -> Result<Vec<PermisoResumen>, ServiceError> {
    let permisos = sqlx::query_as!(
        PermisoResumen,
        "SELECT id, nombre  FROM permisos"
    )
    .fetch_all(pool)
    .await?;

    Ok(permisos)
}