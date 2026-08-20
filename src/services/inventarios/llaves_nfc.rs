use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    models::inventarios::{
        asignaciones_llave::{AsignacionLlave, AsignacionLlaveNueva, AsignacionQuitarLlaveData},
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
                  creado_en, actualizado_en,
                  -- Literal y no un join: una llave recién registrada no puede
                  -- estar asignada, así que NULL no es un atajo, es el valor.
                  NULL::text AS "empleado_codigo?",
                  NULL::uuid AS "asignacion_cerrada_id?"
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
        SELECT ln.id, ln.uid, ln.codigo, ln.estado, ln.descripcion, ln.version,
               ln.creado_en, ln.actualizado_en,
               p.codigo AS "empleado_codigo?",
               -- El read no cierra asignaciones; la columna existe solo para
               -- que el struct sea el mismo en las tres consultas.
               NULL::uuid AS "asignacion_cerrada_id?"
        FROM llaves_nfc ln
        -- `devuelta_en IS NULL` trae la asignación vigente y no todo el
        -- historial; `ux_asignaciones_llave_activa` garantiza que sea a lo sumo
        -- una, así que el LEFT JOIN no duplica filas.
        LEFT JOIN asignaciones_llave a
               ON a.llave_id = ln.id AND a.devuelta_en IS NULL
        LEFT JOIN personal p ON p.id = a.empleado_id
        -- Todas las columnas van calificadas con `ln.`: desde que entró el join
        -- `codigo` existe en las dos tablas y sin el alias es ambiguo.
        WHERE ($1::text IS NULL OR ln.estado = $1)
          -- El uid es UNIQUE y llega normalizado desde el payload, así que es
          -- igualdad exacta: devuelve una llave o ninguna.
          AND ($2::text IS NULL OR ln.uid = $2)
          -- La búsqueda cubre lo que un humano tiene a mano: el código rotulado
          -- en la tarjeta y la descripción. Las llaves sin descripción no se
          -- pierden porque el OR del codigo sigue evaluándose.
          AND ($3::text IS NULL
               OR ln.codigo      ILIKE '%' || $3 || '%'
               OR ln.descripcion ILIKE '%' || $3 || '%')
        ORDER BY ln.creado_en DESC, ln.id DESC
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
        WITH actualizada AS (
            UPDATE llaves_nfc
            SET estado = $3, descripcion = $4
            WHERE id = $1 AND version = $2
            RETURNING id, uid, codigo, estado, descripcion, version,
                      creado_en, actualizado_en
        ), devuelta AS (
            -- Marcar una llave como perdida/dañada/de baja mientras está en
            -- manos de alguien dejaba la asignación abierta: la llave figuraba
            -- con el estado nuevo pero seguía ocupada, y por
            -- `ux_asignaciones_llave_activa` no se le podía dar a nadie más.
            --
            -- `FROM actualizada` la ata al UPDATE de arriba: si el guard de
            -- version no tomó la fila, acá tampoco se cierra nada. Y con $5 NULL
            -- (estado que no obliga a devolver) no matchea ninguna fila.
            UPDATE asignaciones_llave a
            SET devuelta_en = NOW(), motivo_devolucion = $5
            FROM actualizada ln
            WHERE a.llave_id = ln.id
              AND a.devuelta_en IS NULL
              AND $5::text IS NOT NULL
            RETURNING a.id, a.llave_id, a.empleado_id
        )
        -- El join va afuera del UPDATE porque un RETURNING no admite JOIN.
        SELECT ln.id AS "id!", ln.uid AS "uid!", ln.codigo AS "codigo!",
               ln.estado AS "estado!", ln.descripcion,
               ln.version AS "version!", ln.creado_en AS "creado_en!",
               ln.actualizado_en AS "actualizado_en!",
               p.codigo AS "empleado_codigo?",
               d.id     AS "asignacion_cerrada_id?"
        FROM actualizada ln
        LEFT JOIN devuelta d ON d.llave_id = ln.id
        -- Este join ve el snapshot previo (los CTE no ven los cambios de sus
        -- hermanos), así que cubre el caso en que no se cerró nada: la llave
        -- sigue asignada y hay que mostrar a quién.
        LEFT JOIN asignaciones_llave a
               ON a.llave_id = ln.id AND a.devuelta_en IS NULL
        LEFT JOIN personal p ON p.id = COALESCE(d.empleado_id, a.empleado_id)
        "#,
        llave_nfc_id,
        version,
        datos.estado,
        datos.descripcion,
        datos.motivo_cierre,
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
            // Lo unico que puede romper un CHECK acá es el motivo: la tabla solo
            // acepta la lista cerrada del `motivo_valido_check`, y el payload hoy
            // valida largo pero no contenido. Es texto que mandó el cliente, así
            // que 400 y no 500.
            Some("23514") => {
                return ServiceError::BadRequest("el motivo de devolución no es válido".into());
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
pub async fn create_asignaciones_llave(
    conexion: &mut sqlx::PgConnection,
    datos: AsignacionLlaveNueva,
) -> Result<AsignacionLlave, ServiceError> {
    // El `estado = 'inventario'` es lo que le da sentido a marcar una llave como
    // perdida o dañada: sin él la devolución cambia el estado pero la llave
    // sigue siendo asignable, porque quien decide eso son los índices parciales
    // de `asignaciones_llave` y a esos el estado no les dice nada.
    let resultado = sqlx::query_as!(
        AsignacionLlave,
        r#"
        INSERT INTO asignaciones_llave (llave_id, empleado_id)
        SELECT id, $2::uuid
        FROM llaves_nfc
        WHERE uid = $1 AND estado = 'inventario'
        RETURNING id, llave_id, empleado_id, asignada_en, devuelta_en,
                  motivo_devolucion, creado_en
        "#,
        datos.uid,
        datos.empleado_id,
    )
    .fetch_one(&mut *conexion)
    .await;

    match resultado {
        Ok(asignacion) => Ok(asignacion),
        // Si el SELECT no devuelve filas no se inserta nada y el RETURNING
        // vuelve vacío. Con el guard de estado eso ya no significa una sola
        // cosa: puede ser que el uid no exista o que la llave no esté
        // disponible.
        Err(sqlx::Error::RowNotFound) => {
            Err(distinguir_fallo_asignacion(conexion, &datos.uid).await)
        }
        Err(err) => Err(map_conflicto_asignacion(err)),
    }
}

/// Solo corre cuando el INSERT no insertó nada, así que el camino feliz sigue
/// siendo una sola consulta.
async fn distinguir_fallo_asignacion(conexion: &mut sqlx::PgConnection, uid: &str) -> ServiceError {
    let estado = sqlx::query_scalar!("SELECT estado FROM llaves_nfc WHERE uid = $1", uid)
        .fetch_optional(conexion)
        .await;

    match estado {
        // La llave existe pero no está en inventario: el mensaje tiene que
        // decir en qué estado quedó, si no el usuario no sabe qué arreglar.
        Ok(Some(estado)) => ServiceError::Conflict(format!(
            "la llave no está disponible para asignar (estado: {estado})"
        )),
        Ok(None) => ServiceError::NotFound("no existe una llave registrada con ese uid".into()),
        Err(err) => ServiceError::from(err),
    }
}

/// Solo corre cuando el UPDATE no tocó ninguna fila, así que el camino feliz
/// sigue siendo una sola consulta.
async fn distinguir_fallo_devolucion(
    conexion: &mut sqlx::PgConnection,
    asignacion_id: Uuid,
) -> ServiceError {
    let devuelta_en = sqlx::query_scalar!(
        "SELECT devuelta_en FROM asignaciones_llave WHERE id = $1",
        asignacion_id
    )
    .fetch_optional(conexion)
    .await;

    match devuelta_en {
        // La fila existe y ya tiene fecha de devolución: alguien la quitó antes.
        // Es 409 y no 404 porque el id que mandó el cliente es real, lo que ya no
        // corre es la operación.
        Ok(Some(Some(_))) => ServiceError::Conflict("esta llave ya fue devuelta".into()),
        // Sigue activa pese a que el UPDATE no la tomó: dos devoluciones a la vez
        // y la otra transacción todavía no había hecho commit cuando corrimos.
        Ok(Some(None)) => ServiceError::Conflict(
            "la asignación está siendo modificada por otro usuario, intenta de nuevo".into(),
        ),
        Ok(None) => ServiceError::NotFound("la asignación no existe".into()),
        Err(err) => ServiceError::from(err),
    }
}

/// Cierra una asignación activa: le pone fecha de devolución y el motivo.
///
/// No borra la fila ni la reasigna. La asignación es el historial de quién tuvo
/// la llave, y `devuelta_en` es justo lo que los dos índices parciales miran
/// para decidir si la llave y el empleado están libres: al llenarlo, ambos
/// quedan disponibles para una asignación nueva sin tocar nada más.
///
/// Recibe la conexión y no un `E: Executor` genérico porque en la rama de error
/// hace falta una segunda consulta, y un executor genérico se consume en la
/// primera.
pub async fn quitar_asignacion_llave(
    conexion: &mut sqlx::PgConnection,
    datos: AsignacionQuitarLlaveData,
) -> Result<AsignacionLlave, ServiceError> {
    let AsignacionQuitarLlaveData {
        asignacion_id,
        motivo_devolucion,
        estado_llave,
    } = datos;

    // `devuelta_en IS NULL` no es solo un filtro: es el guard que hace la
    // operación idempotente. Sin él, repetir la llamada pisaría la fecha y el
    // motivo de una devolución que ya estaba cerrada.
    //
    // La fecha la pone NOW() y no el cliente: es un hecho del servidor y así
    // nunca puede quedar antes de `asignada_en` (el `fechas_check`).
    let resultado = sqlx::query_as!(
        AsignacionLlave,
        r#"
        WITH cerrada AS (
            UPDATE asignaciones_llave
            SET devuelta_en = NOW(), motivo_devolucion = $2
            WHERE id = $1 AND devuelta_en IS NULL
            RETURNING id, llave_id, empleado_id, asignada_en, devuelta_en,
                      motivo_devolucion, creado_en
        ), _llave AS (
            -- Va en el mismo statement y no en una segunda query para que no
            -- exista un instante en que la asignación esté cerrada y la llave
            -- perdida siga figurando en inventario.
            --
            -- Depende de `cerrada`, así que si el guard de arriba no tomó
            -- ninguna fila esto tampoco toca nada: no hay que repetir la
            -- condición. Y con $3 NULL (motivo que no degrada la llave) no
            -- matchea, así que no reescribe la fila ni le sube la `version`
            -- a nadie por gusto.
            UPDATE llaves_nfc
            SET estado = $3
            FROM cerrada
            WHERE llaves_nfc.id = cerrada.llave_id
              AND $3::text IS NOT NULL
        )
        SELECT id AS "id!", llave_id AS "llave_id!",
               empleado_id AS "empleado_id!", asignada_en AS "asignada_en!",
               devuelta_en, motivo_devolucion, creado_en AS "creado_en!"
        FROM cerrada
        "#,
        asignacion_id,
        motivo_devolucion,
        estado_llave,
    )
    .fetch_one(&mut *conexion)
    .await;

    match resultado {
        Ok(asignacion) => Ok(asignacion),
        // El guard hace que "no existe" y "ya estaba devuelta" lleguen igual,
        // como RowNotFound; hay que separarlos para que el mensaje sirva.
        Err(sqlx::Error::RowNotFound) => {
            Err(distinguir_fallo_devolucion(conexion, asignacion_id).await)
        }
        Err(err) => Err(map_conflicto_asignacion(err)),
    }
}
