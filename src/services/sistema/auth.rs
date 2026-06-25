use sqlx::{self, PgPool};
use uuid::Uuid;

use crate::{models::usuario::Usuario, services::error::ServiceError};

pub async fn get_usuario_username(pool: &PgPool , username:&str) -> 
    Result<Option<Usuario>, ServiceError> {
    let usuario = sqlx::query_as::<_, Usuario>(
        "SELECT * FROM usuarios WHERE usuario = $1 AND activo = true",
    )
    .bind(username)
    .fetch_optional(pool)
    .await?;

    Ok(usuario)
}

pub async fn get_permisos_por_cargo(
    pool: &PgPool,
    cargo_id: Uuid
) -> Result<Vec<String>, ServiceError> {
    let permisos = sqlx::query_scalar::<_, String>(
        r#"
        SELECT p.nombre
        FROM cargos_permisos cp
        JOIN permisos p ON p.id = cp.permiso_id
        WHERE cp.cargo_id = $1
        "#,
    )
    .bind(cargo_id)
    .fetch_all(pool)
    .await?;

    Ok(permisos)
}