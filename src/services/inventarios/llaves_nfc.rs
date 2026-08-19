use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    models::inventarios::{
        asignaciones_llave::{AsignacionLlave, AsignacionLlaveNueva},
        llaves_nfc::{LlaveNfc, LlaveNfcActualizado, LlaveNfcFiltros, LlaveNfcNueva},
    },
    services::error::ServiceError,
};

/// Traduce los errores de integridad de la tabla `llaves_nfc` a errores de
/// negocio; si no, registrar dos veces la misma tarjeta saldría como 500.
fn map_conflicto_llave_nfc(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            // Hay dos UNIQUE en la tabla y el mensaje tiene que decir cuál
            // falló: el del uid lo provoca el usuario y lo puede corregir; el
            // del codigo solo pasa si la secuencia quedó desincronizada.
            Some("23505") => {
                let mensaje = match db_err.constraint() {
                    Some("llaves_nfc_codigo_key") => "ya existe una llave con ese código",
                    _ => "ya existe una llave registrada con ese uid",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            // llaves_nfc_estado_check
            Some("23514") => {
                return ServiceError::Conflict("el estado de la llave no es válido".into());
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

pub async fn add_llave_nfc<'e, E>(
    executor: E,
    datos: LlaveNfcNueva,
) -> Result<LlaveNfc, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // `codigo` y `estado` no se mandan a propósito: los pone la tabla (la
    // secuencia y el DEFAULT 'inventario'), así el consecutivo no depende de
    // lo que mande el cliente y ninguna llave nace ya dada de baja.
    let nueva_llave_nfc = sqlx::query_as!(
        LlaveNfc,
        r#"
        INSERT INTO llaves_nfc (uid, descripcion)
        VALUES ($1, $2)
        RETURNING id, uid, codigo, estado, descripcion, version,
                  creado_en, actualizado_en
        "#,
        datos.uid,
        datos.descripcion
    )
    .fetch_one(executor)
    .await
    .map_err(map_conflicto_llave_nfc)?;

    Ok(nueva_llave_nfc)
}

/// Listado filtrado. El SQL queda estático y cada filtro se apaga con un
/// `$n IS NULL`: así hay un solo plan que Postgres puede cachear y no se arma
/// la consulta concatenando texto.
pub async fn get_llaves_nfc(
    pool: &PgPool,
    filtros: LlaveNfcFiltros,
) -> Result<Vec<LlaveNfc>, ServiceError> {
    let llaves = sqlx::query_as!(
        LlaveNfc,
        r#"
        SELECT id, uid, codigo, estado, descripcion, version,
               creado_en, actualizado_en
        FROM llaves_nfc
        WHERE ($1::text IS NULL OR estado = $1)
          -- El uid es UNIQUE y llega normalizado desde el payload, así que es
          -- igualdad exacta: devuelve una llave o ninguna.
          AND ($2::text IS NULL OR uid = $2)
          -- La búsqueda cubre lo que un humano tiene a mano: el código rotulado
          -- en la tarjeta y la descripción. Las llaves sin descripción no se
          -- pierden porque el OR del codigo sigue evaluándose.
          AND ($3::text IS NULL
               OR codigo      ILIKE '%' || $3 || '%'
               OR descripcion ILIKE '%' || $3 || '%')
        ORDER BY creado_en DESC, id DESC
        LIMIT $4
        "#,
        filtros.estado,
        filtros.uid,
        filtros.busqueda,
        filtros.limite,
    )
    .fetch_all(pool)
    .await?;

    Ok(llaves)
}

/// Solo corre cuando el UPDATE ya falló, así que el camino feliz sigue siendo
/// una sola consulta.
async fn distinguir_fallo_update(
    conexion: &mut sqlx::PgConnection,
    llave_nfc_id: Uuid,
) -> ServiceError {
    let existe = sqlx::query_scalar!(
        "SELECT EXISTS(SELECT 1 FROM llaves_nfc WHERE id = $1)",
        llave_nfc_id
    )
    .fetch_one(conexion)
    .await;

    match existe {
        Ok(Some(true)) => ServiceError::VersionDesactualizada(
            "la llave fue modificada por otro usuario, recarga los datos".into(),
        ),
        Ok(_) => ServiceError::NotFound("llave no encontrada".into()),
        Err(err) => ServiceError::from(err),
    }
}

/// Recibe la conexión en vez de un `E: Executor` genérico porque en la rama de
/// error hace falta consultar una segunda vez, y un executor genérico se
/// consume en la primera query.
pub async fn update_llave_nfc(
    conexion: &mut sqlx::PgConnection,
    actualizado: LlaveNfcActualizado,
) -> Result<LlaveNfc, ServiceError> {
    let LlaveNfcActualizado {
        llave_nfc_id,
        version,
        datos,
    } = actualizado;

    // uid y codigo no están en el SET: identifican la tarjeta física y el
    // consecutivo rotulado, no son datos editables. version tampoco: la sube el
    // trigger de la tabla.
    let resultado = sqlx::query_as!(
        LlaveNfc,
        r#"
        UPDATE llaves_nfc
        SET estado = $3, descripcion = $4
        WHERE id = $1 AND version = $2
        RETURNING id, uid, codigo, estado, descripcion, version,
                  creado_en, actualizado_en
        "#,
        llave_nfc_id,
        version,
        datos.estado,
        datos.descripcion,
    )
    .fetch_one(&mut *conexion)
    .await;

    match resultado {
        Ok(llave_nfc) => Ok(llave_nfc),
        // El guard de version hace que "no existe" y "te ganaron de mano"
        // lleguen igual, como RowNotFound; hay que separarlos.
        Err(sqlx::Error::RowNotFound) => Err(distinguir_fallo_update(conexion, llave_nfc_id).await),
        Err(err) => Err(map_conflicto_llave_nfc(err)),
    }
}

/// Traduce los errores de integridad de `asignaciones_llave`. Los tres casos
/// que se cruzan acá son de negocio, no fallas: la llave ya está en manos de
/// alguien, el empleado ya tiene una, o el empleado no existe.
fn map_conflicto_asignacion(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            // Los dos UNIQUE son parciales (solo filas con devuelta_en NULL) y
            // Postgres reporta el nombre del índice: hay que decir cuál de las
            // dos puntas está ocupada para que el mensaje sirva de algo.
            Some("23505") => {
                let mensaje = match db_err.constraint() {
                    Some("ux_asignaciones_empleado_activa") => {
                        "el empleado ya tiene una llave asignada"
                    }
                    _ => "la llave ya está asignada a otro empleado",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            // La FK de llave_id no puede fallar: el id sale del SELECT de la
            // misma consulta. La que queda es la del empleado.
            Some("23503") => {
                return ServiceError::NotFound("el empleado no existe".into());
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

/// Entrega una llave a un empleado. El payload trae el `uid` que leyó el
/// lector, no el id: la llave se resuelve dentro del mismo INSERT ... SELECT
/// para no gastar una consulta previa y para que el executor genérico alcance.
///
/// `asignada_en` y `creado_en` los pone la tabla; `devuelta_en` y
/// `motivo_devolucion` nacen NULL, que es lo que marca la asignación activa
/// para los índices parciales.
pub async fn create_asignaciones_llave<'e, E>(
    executor: E,
    datos: AsignacionLlaveNueva,
) -> Result<AsignacionLlave, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let asignacion = sqlx::query_as!(
        AsignacionLlave,
        r#"
        INSERT INTO asignaciones_llave (llave_id, empleado_id)
        SELECT id, $2::uuid
        FROM llaves_nfc
        WHERE uid = $1
        RETURNING id, llave_id, empleado_id, asignada_en, devuelta_en,
                  motivo_devolucion, creado_en
        "#,
        datos.uid,
        datos.empleado_id,
    )
    .fetch_one(executor)
    .await
    .map_err(|err| match err {
        // Si el uid no existe el SELECT no devuelve filas, así que no se
        // inserta nada y el RETURNING vuelve vacío: no es una falla de la
        // consulta, es una tarjeta que no está registrada.
        sqlx::Error::RowNotFound => {
            ServiceError::NotFound("no existe una llave registrada con ese uid".into())
        }
        err => map_conflicto_asignacion(err),
    })?;

    Ok(asignacion)
}
