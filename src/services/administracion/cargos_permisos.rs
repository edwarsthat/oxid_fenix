use sqlx::PgPool;
use uuid::Uuid;

use crate::{models::{cargos_permisos::CargosPermisos}, services::error::ServiceError};

pub async fn get_permisos_de_cargo(
    pool: &PgPool,
    cargo_id: Uuid,
) -> Result<Vec<Uuid>, ServiceError> {
    let permisos = sqlx::query_scalar!(
        r#"
        SELECT permiso_id
        FROM cargos_permisos
        WHERE cargo_id = $1
        "#,
        cargo_id,
    )
    .fetch_all(pool)
    .await?;

    Ok(permisos)
}

pub async fn add_cargo_permiso<'e, E>(
    executor: E,
    cargo_id: Uuid,
    permisos: Vec<String>,
) -> Result<Vec<CargosPermisos>, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // Convertimos cada permiso (String) a Uuid
    let permiso_ids: Vec<Uuid> = permisos
        .iter()
        .map(|p| Uuid::parse_str(p))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ServiceError::BadRequest("permiso invalido".into()))?;

    let relaciones = sqlx::query_as!(
        CargosPermisos,
        r#"
        INSERT INTO cargos_permisos (cargo_id, permiso_id)
        SELECT $1, unnest($2::uuid[])
        ON CONFLICT (cargo_id, permiso_id) DO NOTHING
        RETURNING cargo_id, permiso_id
        "#,
        cargo_id,
        &permiso_ids
    )
    .fetch_all(executor)
    .await?;

    Ok(relaciones)
}

pub async fn sync_cargo_permisos(
    conn: &mut sqlx::PgConnection,          // ← concreto, no genérico
    cargo_id: Uuid,
    permisos: Vec<String>,
) -> Result<(), ServiceError> {
    let permiso_ids: Vec<Uuid> = permisos
        .iter()
        .map(|p| Uuid::parse_str(p))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ServiceError::BadRequest("permiso invalido".into()))?;

    // 1) borra los que ya no están
    sqlx::query!(
        r#"
        DELETE FROM cargos_permisos
        WHERE cargo_id = $1
          AND permiso_id <> ALL($2::uuid[])
        "#,
        cargo_id,
        &permiso_ids
    )
    .execute(&mut *conn)   // ← reborrow
    .await?;

    // 2) inserta los que faltan
    sqlx::query!(
        r#"
        INSERT INTO cargos_permisos (cargo_id, permiso_id)
        SELECT $1, unnest($2::uuid[])
        ON CONFLICT (cargo_id, permiso_id) DO NOTHING
        "#,
        cargo_id,
        &permiso_ids
    )
    .execute(&mut *conn)   // ← reborrow otra vez
    .await?;

    Ok(())
}
