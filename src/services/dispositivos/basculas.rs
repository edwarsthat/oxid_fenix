use uuid::Uuid;

use crate::{models::dispositivos::basculas::Bascula, services::error::ServiceError};

/// La báscula activa con este `device_id` que cuelga de este concentrador, o
/// None si no hay ninguna.
///
/// `concentrador_id` es el del concentrador que se autenticó, nunca uno que
/// venga en el cuerpo: así una Pi no puede mandar pesajes a nombre de una
/// báscula de otra.
///
/// El `device_id` se pasa a minúsculas acá porque la tabla lo guarda así y un
/// firmware que mande la MAC en mayúsculas no encontraría su báscula.
pub async fn bascula_por_device_id<'e, E>(
    executor: E,
    device_id: &str,
    concentrador_id: Uuid,
) -> Result<Option<Bascula>, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let bascula = sqlx::query_as!(
        Bascula,
        r#"
        SELECT id, codigo, device_id, concentrador_id, linea, nombre,
               ubicacion, descripcion, estado, version,
               creado_en, actualizado_en
        FROM basculas
        WHERE device_id = $1 AND concentrador_id = $2 AND estado = 'activo'
        "#,
        device_id.to_lowercase(),
        concentrador_id,
    )
    .fetch_optional(executor)
    .await?;

    Ok(bascula)
}
