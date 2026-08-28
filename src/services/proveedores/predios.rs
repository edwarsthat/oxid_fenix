use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    models::proveedores::predios::{PredioActualizado, PredioNuevo, Predios, PrediosFiltros},
    services::error::ServiceError,
};

/// Traduce los errores de integridad de la tabla `predios` a errores de
/// negocio; si no, cargar dos veces la misma finca saldría como 500.
fn map_conflicto_predio(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            // Los dos UNIQUE de la tabla. El del nombre lo provoca el usuario y
            // lo puede corregir; el del codigo solo pasa si la secuencia quedó
            // desincronizada.
            Some("23505") => {
                let mensaje = match db_err.constraint() {
                    Some("predios_codigo_key") => "ya existe un predio con ese código",
                    _ => "el proveedor ya tiene un predio registrado con ese nombre",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            // La única FK de la tabla es la del proveedor: el id llega en el
            // payload, así que un proveedor borrado o mal escrito cae acá.
            Some("23503") => {
                return ServiceError::NotFound("el proveedor indicado no existe".into());
            }
            // Los tres CHECK de la coordenada. El payload ya los valida con las
            // mismas reglas, así que llegar acá significa que alguien insertó
            // sin pasar por `PredioAddPayload`: igual conviene un mensaje que
            // diga qué columna, y no un 23514 pelado.
            Some("23514") => {
                let mensaje = match db_err.constraint() {
                    Some("predios_latitud_check") => "la latitud está fuera de rango",
                    Some("predios_longitud_check") => "la longitud está fuera de rango",
                    _ => "la latitud y la longitud van juntas: falta una de las dos",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

pub async fn add_predios<'e, E>(executor: E, datos: PredioNuevo) -> Result<Predios, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // id, codigo, activo, version, creado_en y actualizado_en se omiten a
    // propósito: los pone la tabla por DEFAULT (el consecutivo sale de la
    // secuencia, no de lo que mande el cliente) y vuelven en el RETURNING.
    //
    // Las coordenadas van y vuelven casteadas: la columna es NUMERIC y el
    // driver no trae tipo decimal habilitado, así que el parámetro entra como
    // float8 y el RETURNING las devuelve igual.
    let nuevo_predio = sqlx::query_as!(
        Predios,
        r#"
        INSERT INTO predios (proveedor_id, nombre, departamento, municipio, vereda,
                             referencia_ubicacion, latitud, longitud,
                             responsable_nombre, responsable_documento, responsable_telefono,
                             observaciones)
        VALUES ($1, $2, $3, $4, $5, $6, $7::float8::numeric, $8::float8::numeric,
                $9, $10, $11, $12)
        RETURNING id, codigo, proveedor_id, nombre, departamento, municipio, vereda,
                  referencia_ubicacion,
                  latitud::float8  AS "latitud",
                  longitud::float8 AS "longitud",
                  responsable_nombre, responsable_documento, responsable_telefono,
                  observaciones, activo, version, creado_en, actualizado_en
        "#,
        datos.proveedor_id,
        datos.nombre,
        datos.departamento,
        datos.municipio,
        datos.vereda,
        datos.referencia_ubicacion,
        datos.latitud,
        datos.longitud,
        datos.responsable_nombre,
        datos.responsable_documento,
        datos.responsable_telefono,
        datos.observaciones,
    )
    .fetch_one(executor)
    .await
    .map_err(map_conflicto_predio)?;

    Ok(nuevo_predio)
}

/// Listado filtrado. El SQL queda estático y cada filtro se apaga con un
/// `$n IS NULL`: así hay un solo plan que Postgres puede cachear y no se arma
/// la consulta concatenando texto.
pub async fn get_predios(
    pool: &PgPool,
    filtros: PrediosFiltros,
) -> Result<Vec<Predios>, ServiceError> {
    // Las coordenadas vuelven casteadas a float8 por lo mismo que en el alta:
    // la columna es NUMERIC y el driver no trae tipo decimal habilitado.
    let predios = sqlx::query_as!(
        Predios,
        r#"
        SELECT id, codigo, proveedor_id, nombre, departamento, municipio, vereda,
               referencia_ubicacion,
               latitud::float8  AS "latitud",
               longitud::float8 AS "longitud",
               responsable_nombre, responsable_documento, responsable_telefono,
               observaciones, activo, version, creado_en, actualizado_en
        FROM predios
        WHERE activo = $1
          -- El filtro de todos los días: los predios de un proveedor al momento
          -- de registrar una recepción. Va por el índice `idx_predios_proveedor`.
          AND ($2::uuid IS NULL OR proveedor_id = $2)
          -- Igualdad, no un LIKE: el departamento se filtra por el valor
          -- completo. Va por `lower()` en los dos lados porque la columna se
          -- guarda como la escribieron y "Antioquia" tiene que encontrar
          -- también a "ANTIOQUIA".
          AND ($3::text IS NULL OR lower(departamento) = lower($3))
          -- La búsqueda cubre lo que un humano tiene a mano del predio: cómo le
          -- dicen a la finca, el consecutivo, dónde queda y quién la atiende.
          -- Los que no tienen vereda ni responsable no se pierden: los otros OR
          -- se siguen evaluando.
          AND ($4::text IS NULL
               OR nombre             ILIKE '%' || $4 || '%'
               OR codigo             ILIKE '%' || $4 || '%'
               OR municipio          ILIKE '%' || $4 || '%'
               OR vereda             ILIKE '%' || $4 || '%'
               OR responsable_nombre ILIKE '%' || $4 || '%')
        ORDER BY creado_en DESC, id DESC
        LIMIT $5
        "#,
        filtros.activo,
        filtros.proveedor_id,
        filtros.departamento,
        filtros.busqueda,
        filtros.limite,
    )
    .fetch_all(pool)
    .await?;

    Ok(predios)
}

/// El UPDATE lleva un guard por `version`, así que "no existe" y "te ganaron de
/// mano" llegan igual, como RowNotFound: hay que separarlos para decirle al
/// usuario si recarga o si el predio ya no está.
async fn distinguir_fallo_update(
    conexion: &mut sqlx::PgConnection,
    predio_id: Uuid,
) -> ServiceError {
    let existe = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM predios WHERE id = $1)",
        predio_id
    )
    .fetch_one(conexion)
    .await;

    match existe {
        Ok(Some(true)) => ServiceError::VersionDesactualizada(
            "El predio fue modificado por otro usuario, recarga los datos".into(),
        ),
        Ok(_) => ServiceError::NotFound("Predio no encontrado".into()),
        Err(err) => ServiceError::from(err),
    }
}

pub async fn update_predio(
    conexion: &mut sqlx::PgConnection,
    actualizado: PredioActualizado,
) -> Result<Predios, ServiceError> {
    let PredioActualizado {
        predio_id,
        version,
        datos,
    } = actualizado;

    // codigo y activo quedan fuera del SET: el consecutivo no se reescribe y
    // dar de baja es otra operación. version tampoco se toca: la sube el
    // trigger de la tabla. proveedor_id sí entra: un predio se puede reasignar.
    //
    // Las coordenadas van y vuelven casteadas, igual que en el alta: la columna
    // es NUMERIC y el driver no trae tipo decimal habilitado.
    let resultado = sqlx::query_as!(
        Predios,
        r#"
        UPDATE predios
        SET proveedor_id = $3, nombre = $4, departamento = $5, municipio = $6, vereda = $7,
            referencia_ubicacion = $8,
            latitud = $9::float8::numeric, longitud = $10::float8::numeric,
            responsable_nombre = $11, responsable_documento = $12, responsable_telefono = $13,
            observaciones = $14
        WHERE id = $1 AND version = $2
        RETURNING id, codigo, proveedor_id, nombre, departamento, municipio, vereda,
                  referencia_ubicacion,
                  latitud::float8  AS "latitud",
                  longitud::float8 AS "longitud",
                  responsable_nombre, responsable_documento, responsable_telefono,
                  observaciones, activo, version, creado_en, actualizado_en
        "#,
        predio_id,
        version,
        datos.proveedor_id,
        datos.nombre,
        datos.departamento,
        datos.municipio,
        datos.vereda,
        datos.referencia_ubicacion,
        datos.latitud,
        datos.longitud,
        datos.responsable_nombre,
        datos.responsable_documento,
        datos.responsable_telefono,
        datos.observaciones,
    )
    .fetch_one(&mut *conexion)
    .await;

    match resultado {
        Ok(predio) => Ok(predio),
        Err(sqlx::Error::RowNotFound) => Err(distinguir_fallo_update(conexion, predio_id).await),
        Err(err) => Err(map_conflicto_predio(err)),
    }
}

/// Baja lógica: el predio queda con `activo = FALSE` y no se borra la fila,
/// porque las recepciones que ya se le cargaron lo siguen referenciando (la FK
/// del proveedor es ON DELETE RESTRICT por lo mismo).
pub async fn delete_predio<'e, E>(executor: E, predio_id: Uuid) -> Result<Predios, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // El guard `activo = TRUE` hace idempotente la operación: dar de baja dos
    // veces no vuelve a mover `actualizado_en` ni a subir la version, y la
    // segunda vez la fila no sale del UPDATE.
    let predio_retirado = sqlx::query_as!(
        Predios,
        r#"
        UPDATE predios
        SET activo = FALSE
        WHERE id = $1 AND activo = TRUE
        RETURNING id, codigo, proveedor_id, nombre, departamento, municipio, vereda,
                  referencia_ubicacion,
                  latitud::float8  AS "latitud",
                  longitud::float8 AS "longitud",
                  responsable_nombre, responsable_documento, responsable_telefono,
                  observaciones, activo, version, creado_en, actualizado_en
        "#,
        predio_id,
    )
    .fetch_one(executor)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => {
            ServiceError::NotFound("predio no encontrado o ya está inactivo".into())
        }
        err => map_conflicto_predio(err),
    })?;

    Ok(predio_retirado)
}

/// Reactivación: revierte la baja lógica de [`delete_predio`].
///
/// Solo se toca `activo`; el resto de los datos quedan como estaban al darlo de
/// baja, porque el predio que vuelve es el mismo y las recepciones que ya se le
/// cargaron siguen colgadas de esta fila. Si algo cambió mientras estuvo
/// inactivo, eso es un `update` aparte.
///
/// El guard `activo = FALSE` hace idempotente la operación: reactivar a uno que
/// ya está activo no mueve `actualizado_en` ni sube la version, y la fila no
/// sale del UPDATE.
pub async fn activar_predio<'e, E>(executor: E, predio_id: Uuid) -> Result<Predios, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let predio_activado = sqlx::query_as!(
        Predios,
        r#"
        UPDATE predios
        SET activo = TRUE
        WHERE id = $1 AND activo = FALSE
        RETURNING id, codigo, proveedor_id, nombre, departamento, municipio, vereda,
                  referencia_ubicacion,
                  latitud::float8  AS "latitud",
                  longitud::float8 AS "longitud",
                  responsable_nombre, responsable_documento, responsable_telefono,
                  observaciones, activo, version, creado_en, actualizado_en
        "#,
        predio_id,
    )
    .fetch_one(executor)
    .await
    .map_err(|err| match err {
        sqlx::Error::RowNotFound => {
            ServiceError::NotFound("predio no encontrado o ya está activo".into())
        }
        err => map_conflicto_predio(err),
    })?;

    Ok(predio_activado)
}
