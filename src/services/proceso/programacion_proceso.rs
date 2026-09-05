use uuid::Uuid;

use crate::{
    models::proceso::programacion_proceso::{
        AltaProgramacionProceso, ProgramacionProceso, ProgramacionProcesoNuevo,
    },
    services::error::ServiceError,
};

/// La única línea de la planta. Es el mismo valor que el DEFAULT de la columna
/// `linea`, repetido acá porque el alta necesita saber cuál programación
/// cerrar y un UPDATE no puede leer el DEFAULT de otra fila.
///
/// El día que exista la segunda línea, esto deja de ser una constante y pasa a
/// ser un dato del payload; el resto de la función no cambia.
const LINEA_UNICA: i16 = 1;

/// Traduce los errores de integridad de `programaciones_proceso` a errores de
/// negocio; si no, programar un lote que ya no existe saldría como 500.
fn map_conflicto_programacion(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            // El único UNIQUE de la tabla es `ux_programaciones_proceso_abierta`,
            // o sea el invariante de "una sola abierta". Llegar acá no es un
            // dato malo: son dos coordinadores programando a la vez y este
            // perdió la carrera, así que lo que hay que decirle es que mire
            // qué quedó montado antes de volver a intentar.
            Some("23505") => {
                return ServiceError::Conflict(
                    "alguien más acaba de programar un lote en la línea; recargue para ver cuál"
                        .into(),
                );
            }
            // El lote lo manda el cliente, así que el mensaje tiene que decir
            // cuál id está mal. `programado_por` sale de la sesión: si falla,
            // el usuario se borró con la sesión viva.
            Some("23503") => {
                let mensaje = match db_err.constraint() {
                    Some("programaciones_proceso_programado_por_fkey") => {
                        "el usuario que programa ya no existe"
                    }
                    _ => "el lote indicado no existe",
                };
                return ServiceError::NotFound(mensaje.into());
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

/// Alta de una programación: qué lote se monta en la línea.
///
/// `programado_por` va aparte de `datos` a propósito: sale de la sesión
/// (`ctx.user_id`) y no del payload, así que no puede viajar dentro de algo que
/// armó el cliente.
///
/// Recibe una conexión y no un executor genérico porque son cuatro consultas y
/// tienen que ir en la misma transacción: un `E: Executor` se consume en la
/// primera. Y tienen que ir juntas porque entre cerrar la anterior e insertar
/// la nueva no puede existir un instante sin ninguna abierta — en ese hueco,
/// una pesada de la báscula no tendría lote al que colgarse.
///
/// Es idempotente sin clave, por la forma de lo que guarda: una programación es
/// un estado ("la línea está en el lote A"), no un evento. Si el coordinador
/// reintenta porque se le perdió la respuesta, y la abierta ya es ese mismo
/// lote, se devuelve esa con `creado: false` en vez de cerrarla y reabrirla —
/// que partiría el intervalo en dos y dejaría escrito que el lote se
/// interrumpió y se retomó cuando nunca pasó.
pub async fn add_programacion_proceso(
    conexion: &mut sqlx::PgConnection,
    datos: ProgramacionProcesoNuevo,
    programado_por: Uuid,
) -> Result<AltaProgramacionProceso, ServiceError> {
    // Lo que está corriendo ahora, si hay algo. El índice parcial garantiza que
    // sea una sola fila, así que no hace falta ordenar ni limitar.
    let abierta = sqlx::query_as!(
        ProgramacionProceso,
        r#"
        SELECT id, lote_id, linea, inicio_en, fin_en, programado_por,
               cerrado_por, observaciones, version, creado_en, actualizado_en
        FROM programaciones_proceso
        WHERE linea = $1 AND fin_en IS NULL
        "#,
        LINEA_UNICA,
    )
    .fetch_optional(&mut *conexion)
    .await
    .map_err(map_conflicto_programacion)?;

    // De la anterior, más adelante, solo hace falta el id para cerrarla: se
    // resuelve todo acá y así la fila entera queda libre para devolverla en el
    // caso del reintento.
    let anterior_id = match abierta {
        // El reintento: ya está montado el lote que se pide. El estado que
        // quería el cliente es el que hay, así que esto termina bien sin tocar
        // nada.
        Some(actual) if actual.lote_id == datos.lote_id => {
            return Ok(AltaProgramacionProceso {
                programacion: actual,
                creado: false,
            });
        }
        Some(actual) => Some(actual.id),
        None => None,
    };

    // La FK ya garantiza que el lote exista, pero no dice nada de su estado, y
    // montar un lote anulado o sin saldo mandaría las pesadas del turno a un
    // lote que no puede recibirlas. El código va en el mensaje porque es lo que
    // el coordinador tiene escrito en la estiba, no el UUID.
    let lote = sqlx::query!(
        r#"
        SELECT codigo, anulado_en, cerrado_en
        FROM ingresos_materia_prima
        WHERE id = $1
        "#,
        datos.lote_id,
    )
    .fetch_optional(&mut *conexion)
    .await
    .map_err(map_conflicto_programacion)?
    .ok_or_else(|| ServiceError::NotFound("el lote indicado no existe".into()))?;

    if lote.anulado_en.is_some() {
        return Err(ServiceError::Conflict(format!(
            "el lote {} está anulado y no se puede procesar",
            lote.codigo
        )));
    }

    if lote.cerrado_en.is_some() {
        return Err(ServiceError::Conflict(format!(
            "el lote {} ya no tiene saldo en patio",
            lote.codigo
        )));
    }

    // Programar un lote nuevo es, implícitamente, terminar con el anterior: por
    // eso `cerrado_por` es el mismo que programa. El `fin_en IS NULL` del WHERE
    // no sobra aunque el id ya identifique la fila — si otra transacción la
    // cerró mientras tanto, este UPDATE no hace nada y el INSERT de abajo se
    // encarga de que la carrera salga por el 23505 y no pisando un cierre ajeno.
    if let Some(anterior_id) = anterior_id {
        sqlx::query!(
            r#"
            UPDATE programaciones_proceso
            SET fin_en = NOW(), cerrado_por = $2
            WHERE id = $1 AND fin_en IS NULL
            "#,
            anterior_id,
            programado_por,
        )
        .execute(&mut *conexion)
        .await
        .map_err(map_conflicto_programacion)?;
    }

    // `linea` e `inicio_en` se omiten a propósito: las dos tienen DEFAULT, y la
    // hora la pone el reloj del servidor de base y no el de este proceso. El
    // cierre (`fin_en`, `cerrado_por`) y `version` tampoco van: son de otra
    // operación y del trigger.
    let programacion = sqlx::query_as!(
        ProgramacionProceso,
        r#"
        INSERT INTO programaciones_proceso (lote_id, programado_por, observaciones)
        VALUES ($1, $2, $3)
        RETURNING id, lote_id, linea, inicio_en, fin_en, programado_por,
                  cerrado_por, observaciones, version, creado_en, actualizado_en
        "#,
        datos.lote_id,
        programado_por,
        datos.observaciones,
    )
    .fetch_one(&mut *conexion)
    .await
    .map_err(map_conflicto_programacion)?;

    Ok(AltaProgramacionProceso {
        programacion,
        creado: true,
    })
}
