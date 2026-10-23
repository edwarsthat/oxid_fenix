use crate::{models::dispositivos::concentradores::Concentrador, services::error::ServiceError};

/// El concentrador activo dueño de esta llave, o None si no hay ninguno.
/// Recibe el hash y no la llave: la llave en claro no sale del middleware.
pub async fn concentrador_por_llave_hash<'e, E>(
    executor: E,
    llave_hash: &str,
) -> Result<Option<Concentrador>, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let concentrador = sqlx::query_as!(
        Concentrador,
        r#"
        SELECT id, codigo, nombre, ubicacion, descripcion, llave_hash,
               llave_prefijo, llave_emitida_en, estado, version,
               creado_en, actualizado_en
        FROM concentradores
        WHERE llave_hash = $1 AND estado = 'activo'
        "#,
        llave_hash,
    )
    .fetch_optional(executor)
    .await?;

    Ok(concentrador)
}
