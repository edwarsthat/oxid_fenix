use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    models::administracion::cargos_personal::CargoPersonal, services::error::ServiceError,
};

fn map_conflicto_cargo_personal(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some("23505") {
            return ServiceError::Conflict("ya existe un cargo de personal con ese nombre".into());
        }
    }
    ServiceError::from(err)
}

pub async fn get_cargo_personal(pool: &PgPool) -> Result<Vec<CargoPersonal>, ServiceError> {
    let cargos_personal = sqlx::query_as!(
        CargoPersonal,
        "
        SELECT id, nombre, tipo_contrato, creado_en, activo
        FROM cargos_personal
        "
    )
    .fetch_all(pool)
    .await?;

    Ok(cargos_personal)
}

pub async fn create_cargo_personal<'e, E>(
    executor: E,
    nombre: &str,
    tipo_contrato: &str,
) -> Result<CargoPersonal, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let nuevo_cargo_personal = sqlx::query_as!(
        CargoPersonal,
        r#"
        INSERT INTO cargos_personal (nombre, tipo_contrato)
        VALUES ($1, $2)
        RETURNING id, nombre, tipo_contrato as "tipo_contrato!", creado_en, activo
        "#,
        nombre,
        tipo_contrato
    )
    .fetch_one(executor)
    .await
    .map_err(map_conflicto_cargo_personal)?;

    Ok(nuevo_cargo_personal)
}

pub async fn update_cargo_personal<'e, E>(
    executor: E,
    nombre: &str,
    tipo_contrato: &str,
    cargo_id: Uuid,
) -> Result<CargoPersonal, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let cargo = sqlx::query_as!(
        CargoPersonal,
        r#"
        UPDATE cargos_personal
        SET nombre = $1, tipo_contrato = $2
        WHERE id = $3
        RETURNING id, nombre, tipo_contrato as "tipo_contrato!", creado_en, activo
        "#,
        nombre,
        tipo_contrato,
        cargo_id
    )
    .fetch_one(executor)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => {
            ServiceError::NotFound("cargo de personal no encontrado".into())
        }
        err => map_conflicto_cargo_personal(err),
    })?;

    Ok(cargo)
}

pub async fn delete_cargo_personal<'e, E>(executor: E, cargo_id: Uuid) -> Result<(), ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    sqlx::query_scalar!(
        r#"
        UPDATE cargos_personal
        SET activo = FALSE
        WHERE id = $1
        RETURNING id
        "#,
        cargo_id
    )
    .fetch_one(executor)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => {
            ServiceError::NotFound("cargo de personal no encontrado".into())
        }
        err => ServiceError::from(err),
    })?;

    Ok(())
}

pub async fn activar_cargo_personal<'e, E>(
    executor: E,
    cargo_id: Uuid,
) -> Result<CargoPersonal, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let cargo = sqlx::query_as!(
        CargoPersonal,
        r#"
        UPDATE cargos_personal
        SET activo = TRUE
        WHERE id = $1 AND activo = FALSE
        RETURNING id, nombre, tipo_contrato as "tipo_contrato!", creado_en, activo
        "#,
        cargo_id
    )
    .fetch_one(executor)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => {
            ServiceError::NotFound("cargo de personal no encontrado o ya esta activo".into())
        }
        err => ServiceError::from(err),
    })?;

    Ok(cargo)
}