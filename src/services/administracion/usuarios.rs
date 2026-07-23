use sqlx::PgPool;

use crate::{models::usuario::UsuarioListItem, services::error::ServiceError};

pub async fn get_usuarios(pool: &PgPool) -> Result<Vec<UsuarioListItem>, ServiceError> {
    let usuarios = sqlx::query_as!(
        UsuarioListItem,
        "
        SELECT id, nombre, apellido, email, usuario, cargo_id, activo, creado_en, actualizado_en
        FROM usuarios
        "
    )
    .fetch_all(pool)
    .await?;

    Ok(usuarios)
}
