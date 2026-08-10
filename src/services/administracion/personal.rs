use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    models::administracion::personal::{
        Empleado, PersonalActualizado, PersonalFiltros, PersonalNuevo, TipoFecha,
    },
    services::error::ServiceError,
};

/// Traduce los errores de integridad de la tabla `personal` a errores de
/// negocio; si no, un documento repetido saldría como 500.
fn map_conflicto_personal(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            // uq_personal_documento
            Some("23505") => {
                return ServiceError::Conflict(
                    "ya existe un empleado con ese tipo y número de documento".into(),
                );
            }
            // FK contra cargos_personal
            Some("23503") => {
                return ServiceError::NotFound("el cargo indicado no existe".into());
            }
            // personal_fechas_check
            Some("23514") => {
                return ServiceError::Conflict(
                    "la fecha de retiro no puede ser anterior a la de ingreso".into(),
                );
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

/// Solo corre cuando el UPDATE ya falló, así que el camino feliz sigue siendo
/// una sola consulta.
async fn distinguir_fallo_update(
    conexion: &mut sqlx::PgConnection,
    empleado_id: Uuid,
) -> ServiceError {
    let existe = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM personal WHERE id = $1)",
        empleado_id
    )
    .fetch_one(conexion)
    .await;

    match existe {
        Ok(Some(true)) => ServiceError::VersionDesactualizada(
            "el empleado fue modificado por otro usuario, recarga los datos".into(),
        ),
        Ok(_) => ServiceError::NotFound("empleado no encontrado".into()),
        Err(err) => ServiceError::from(err),
    }
}

pub async fn get_personal(
    pool: &PgPool,
    filtros: PersonalFiltros,
) -> Result<Vec<Empleado>, ServiceError> {
    // El selector se resuelve acá: el SQL queda estático y cada columna se
    // compara contra su propio parámetro (sin CASE, sin perder los índices).
    let desde = filtros.fecha.and_then(|r| r.desde());
    let hasta = filtros.fecha.and_then(|r| r.hasta());

    let (ingreso_desde, ingreso_hasta, retiro_desde, retiro_hasta) = match filtros.tipo_fecha {
        TipoFecha::Ingreso => (desde, hasta, None, None),
        TipoFecha::Retiro => (None, None, desde, hasta),
    };

    let personal = sqlx::query_as!(
        Empleado,
        r#"
        SELECT id, codigo, tipo_documento, documento, nombre, apellido,
               fecha_nacimiento, telefono, cargo_id, fecha_ingreso,
               fecha_retiro, activo, version, creado_en, actualizado_en
        FROM personal
        WHERE activo = $1
          AND ($2::uuid IS NULL OR cargo_id = $2)
          AND ($3::text IS NULL
               OR nombre    ILIKE '%' || $3 || '%'
               OR apellido  ILIKE '%' || $3 || '%'
               OR documento ILIKE $3 || '%')
          AND ($4::date IS NULL OR fecha_ingreso >= $4)
          AND ($5::date IS NULL OR fecha_ingreso <= $5)
          AND ($6::date IS NULL OR fecha_retiro >= $6)
          AND ($7::date IS NULL OR fecha_retiro <= $7)
        ORDER BY creado_en DESC, id DESC
        LIMIT $8
        "#,
        filtros.activo,
        filtros.cargo_id,
        filtros.busqueda.as_deref(),
        ingreso_desde,
        ingreso_hasta,
        retiro_desde,
        retiro_hasta,
        filtros.limite,
    )
    .fetch_all(pool)
    .await?;

    Ok(personal)
}

pub async fn add_personal<'e, E>(
    executor: E,
    datos: PersonalNuevo,
) -> Result<Empleado, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // id, codigo, activo, version, creado_en y actualizado_en se omiten a
    // propósito: los pone la tabla por DEFAULT y vuelven en el RETURNING.
    let nuevo_empleado = sqlx::query_as!(
        Empleado,
        r#"
        INSERT INTO personal (tipo_documento, documento, nombre, apellido,
                              fecha_nacimiento, telefono, cargo_id, fecha_ingreso)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id, codigo, tipo_documento, documento, nombre, apellido,
                  fecha_nacimiento, telefono, cargo_id, fecha_ingreso,
                  fecha_retiro, activo, version, creado_en, actualizado_en
        "#,
        datos.tipo_documento,
        datos.documento,
        datos.nombre,
        datos.apellido,
        datos.fecha_nacimiento,
        datos.telefono,
        datos.cargo_id,
        datos.fecha_ingreso,
    )
    .fetch_one(executor)
    .await
    .map_err(map_conflicto_personal)?;

    Ok(nuevo_empleado)
}

/// Recibe la conexión en vez de un `E: Executor` genérico porque en la rama de
/// error hace falta consultar una segunda vez, y un executor genérico se
/// consume en la primera query.
pub async fn update_personal(
    conexion: &mut sqlx::PgConnection,
    actualizado: PersonalActualizado,
) -> Result<Empleado, ServiceError> {
    let PersonalActualizado {
        empleado_id,
        version,
        datos,
    } = actualizado;

    // version no se toca en el SET: la sube el trigger de la tabla.
    let resultado = sqlx::query_as!(
        Empleado,
        r#"
        UPDATE personal
        SET tipo_documento = $3, documento = $4, nombre = $5, apellido = $6,
            fecha_nacimiento = $7, telefono = $8, cargo_id = $9, fecha_ingreso = $10
        WHERE id = $1 AND version = $2
        RETURNING id, codigo, tipo_documento, documento, nombre, apellido,
                  fecha_nacimiento, telefono, cargo_id, fecha_ingreso,
                  fecha_retiro, activo, version, creado_en, actualizado_en
        "#,
        empleado_id,
        version,
        datos.tipo_documento,
        datos.documento,
        datos.nombre,
        datos.apellido,
        datos.fecha_nacimiento,
        datos.telefono,
        datos.cargo_id,
        datos.fecha_ingreso,
    )
    .fetch_one(&mut *conexion)
    .await;

    match resultado {
        Ok(empleado) => Ok(empleado),
        // El guard de version hace que "no existe" y "te ganaron de mano"
        // lleguen igual, como RowNotFound; hay que separarlos.
        Err(sqlx::Error::RowNotFound) => Err(distinguir_fallo_update(conexion, empleado_id).await),
        Err(err) => Err(map_conflicto_personal(err)),
    }
}

/// Baja lógica: marca el retiro en vez de borrar la fila, para no romper las
/// referencias históricas del empleado.
///
/// El retiro queda con la fecha del servidor. El guard `activo = TRUE` evita
/// que una segunda baja pise la fecha de retiro que ya quedó registrada.
///
/// Devuelve la fila y no `()` porque `fecha_retiro` y `version` los decide el
/// servidor: sin el RETURNING, quien llama no tiene cómo saberlos.
pub async fn delete_personal<'e, E>(
    executor: E,
    empleado_id: Uuid,
) -> Result<Empleado, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let empleado_retirado = sqlx::query_as!(
        Empleado,
        r#"
        UPDATE personal
        SET activo = FALSE,
            fecha_retiro = CURRENT_DATE
        WHERE id = $1 AND activo = TRUE
        RETURNING id, codigo, tipo_documento, documento, nombre, apellido,
                  fecha_nacimiento, telefono, cargo_id, fecha_ingreso,
                  fecha_retiro, activo, version, creado_en, actualizado_en
        "#,
        empleado_id,
    )
    .fetch_one(executor)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => {
            ServiceError::NotFound("empleado no encontrado o ya está inactivo".into())
        }
        // personal_fechas_check: salta si el empleado tiene fecha_ingreso
        // futura, o sea que hoy sería anterior a su ingreso.
        err => map_conflicto_personal(err),
    })?;

    Ok(empleado_retirado)
}

/// Reingreso: revierte la baja lógica de [`delete_personal`].
///
/// `fecha_ingreso` se pisa con la fecha del servidor porque el empleado vuelve
/// a entrar hoy; la fecha del contrato anterior queda en el audit log. Limpiar
/// `fecha_retiro` en el mismo SET evita dejar la fila con un retiro anterior al
/// nuevo ingreso, que `personal_fechas_check` rechazaría.
///
/// El guard `activo = FALSE` hace que reactivar dos veces no reinicie la fecha
/// de ingreso de alguien que ya está trabajando.
pub async fn activar_personal<'e, E>(
    executor: E,
    empleado_id: Uuid,
) -> Result<Empleado, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let empleado_activado = sqlx::query_as!(
        Empleado,
        r#"
        UPDATE personal
        SET activo = TRUE,
            fecha_ingreso = CURRENT_DATE,
            fecha_retiro = NULL
        WHERE id = $1 AND activo = FALSE
        RETURNING id, codigo, tipo_documento, documento, nombre, apellido,
                  fecha_nacimiento, telefono, cargo_id, fecha_ingreso,
                  fecha_retiro, activo, version, creado_en, actualizado_en
        "#,
        empleado_id,
    )
    .fetch_one(executor)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => {
            ServiceError::NotFound("empleado no encontrado o ya está activo".into())
        }
        err => ServiceError::from(err),
    })?;

    Ok(empleado_activado)
}