use crate::{models::cargo::Cargo, services::error::ServiceError};
use sqlx::PgPool;
use uuid::Uuid;

pub async fn get_cargos(pool: &PgPool) -> Result<Vec<Cargo>, ServiceError> {
    let cargos = sqlx::query_as!(
        Cargo,
        "SELECT id, nombre, descripcion, creado_en, activo FROM cargos"
    )
    .fetch_all(pool)
    .await?;

    Ok(cargos)
}

pub async fn create_cargo<'e, E>(
    executor: E,
    nombre: &str,
    descripcion: &str,
) -> Result<Cargo, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let cargo = sqlx::query_as!(
        Cargo,
        r#"
        INSERT INTO cargos (nombre, descripcion)
        VALUES ($1, $2)
        RETURNING id, nombre, descripcion, creado_en, activo
        "#,
        nombre,
        descripcion
    )
    .fetch_one(executor)
    .await?;

    Ok(cargo)
}

pub async fn update_cargo<'e, E>(
    executor: E,
    nombre: &str,
    descripcion: &str,
    cargo_id: Uuid,
) -> Result<Cargo, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let cargo = sqlx::query_as!(
        Cargo,
        r#"
        UPDATE cargos
        SET nombre = $1, descripcion = $2
        WHERE id = $3
        RETURNING id, nombre, descripcion, creado_en, activo
        "#,
        nombre,
        descripcion,
        cargo_id
    )
    .fetch_one(executor)
    .await?;

    Ok(cargo)
}

pub async fn soft_delete_cargo<'e, E>(executor: E, cargo_id: Uuid) -> Result<(), ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_scalar!(
        r#"
        UPDATE cargos
        SET activo = FALSE
        WHERE id = $1
        RETURNING id
        "#,
        cargo_id
    )
    .fetch_one(executor)
    .await?;

    Ok(())
}
