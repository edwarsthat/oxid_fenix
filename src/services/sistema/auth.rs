use sqlx::{self, PgPool};

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