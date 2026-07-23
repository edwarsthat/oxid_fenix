use sqlx::PgPool;
use uuid::Uuid;

use crate::{models::usuario::UsuarioListItem, services::error::ServiceError};

pub async fn get_usuarios(pool: &PgPool) -> Result<Vec<UsuarioListItem>, ServiceError> {
    let usuarios = sqlx::query_as!(
        UsuarioListItem,
        "
        SELECT id, nombre, apellido, email, usuario, cargo_id, activo, creado_en, actualizado_en, debe_cambiar_password
        FROM usuarios
        "
    )
    .fetch_all(pool)
    .await?;

    Ok(usuarios)
}


pub async fn create_usuario<'e, E>(
    executor: E,
    nombre: &str,
    apellido: &str,
    email: &str,
    usuario: &str,
    password_hash: &str,
    cargo_id: Uuid,
) -> Result<UsuarioListItem, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let nuevo_usuario = sqlx::query_as!(
        UsuarioListItem,
        r#"
        INSERT INTO usuarios (nombre, apellido, email, usuario, password_hash, cargo_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id, nombre, apellido, email, usuario, cargo_id, activo, creado_en, actualizado_en, debe_cambiar_password
        "#,
        nombre,
        apellido,
        email,
        usuario,
        password_hash,
        cargo_id
    )
    .fetch_one(executor)
    .await
    .map_err(map_conflicto_usuario)?;

    Ok(nuevo_usuario)
}

fn map_conflicto_usuario(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.constraint() {
            Some("usuarios_email_key") => {
                return ServiceError::Conflict("ya existe un usuario con ese email".into());
            }
            Some("usuarios_usuario_key") => {
                return ServiceError::Conflict(
                    "ya existe un usuario con ese nombre de usuario".into(),
                );
            }
            _ => {}
        }
        if db_err.code().as_deref() == Some("23503") {
            return ServiceError::BadRequest("cargo_id no valido".into());
        }
    }
    ServiceError::from(err)
}