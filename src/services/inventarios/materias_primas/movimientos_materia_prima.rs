use uuid::Uuid;

use crate::{
    models::inventarios::materia_prima::movimientos_materia_prima::{
        AltaMovimiento, ConsumoBasculaNuevo, MovimientosMateriaPrima,
    },
    services::error::ServiceError,
};

/// Traduce los errores de integridad de `movimientos_materia_prima` a errores
/// de negocio. En un consumo de báscula ningún id lo escribe una persona: los
/// resuelve el servidor, así que caer acá es casi siempre que algo cambió
/// entre resolver e insertar (un lote o una báscula borrados) o un bug.
fn map_conflicto_movimiento(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            Some("23503") => {
                let mensaje = match db_err.constraint() {
                    Some("movimientos_materia_prima_lote_id_fkey") => "el lote indicado no existe",
                    // La FK compuesta: la programación existe pero es de otro lote.
                    Some("movimientos_materia_prima_programacion_fk") => {
                        "la programación no corresponde a ese lote"
                    }
                    Some("movimientos_materia_prima_operario_id_fkey") => {
                        "el operario indicado no existe"
                    }
                    Some("movimientos_materia_prima_registrado_por_fkey") => {
                        "el usuario que registra el movimiento ya no existe"
                    }
                    Some("movimientos_materia_prima_concentrador_id_fkey") => {
                        "el concentrador indicado no existe"
                    }
                    Some("movimientos_materia_prima_bascula_id_fkey") => {
                        "la báscula indicada no existe"
                    }
                    _ => "el movimiento apunta a un registro que no existe",
                };
                return ServiceError::NotFound(mensaje.into());
            }
            Some("23514") => {
                let mensaje = match db_err.constraint() {
                    Some("movimientos_materia_prima_peso_check") => {
                        "el peso debe ser mayor que cero"
                    }
                    Some("movimientos_materia_prima_origen_bascula_completo_check") => {
                        "un pesaje de báscula tiene que traer todo su origen"
                    }
                    _ => "el movimiento no cumple una regla de la tabla",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

/// Alta de un consumo que llegó desde una báscula.
///
/// `registrado_por` va aparte de `datos` por la misma razón que en las demás
/// altas: no sale de lo que mandó el cliente. Acá es el usuario de sistema de
/// las básculas, porque ninguna persona metió el pesaje.
///
/// Es idempotente por dos lados: `clave_idempotencia` (el `id` de la Pi) y la
/// terna `(bascula_id, sesion_arranque, transaccion)` del ESP32, que atrapa el
/// reenvío con UUID nuevos si la Pi perdió su base. Si choca con cualquiera de
/// las dos, devuelve el movimiento que ya estaba con `creado: false`.
pub async fn add_consumo_bascula(
    conexion: &mut sqlx::PgConnection,
    datos: ConsumoBasculaNuevo,
    registrado_por: Uuid,
) -> Result<AltaMovimiento, ServiceError> {
    // El peso entra como float8 y lo castea la columna: NUMERIC(12,2) redondea
    // a dos decimales al guardar, así que 22.456 de la báscula queda 22.46.
    //
    // ON CONFLICT sin columna a propósito: así cubre los dos UNIQUE (la clave
    // y el índice parcial de la terna). Con `(clave_idempotencia)` el choque
    // de la terna saldría como 23505 en vez de como reintento.
    let insertado = sqlx::query_as!(
        MovimientosMateriaPrima,
        r#"
        INSERT INTO movimientos_materia_prima
            (clave_idempotencia, lote_id, programacion_id, tipo, peso,
             operario_id, registrado_por,
             medido_en, recibido_concentrador_en, concentrador_id, bascula_id,
             sesion_arranque, transaccion, uid_tarjeta)
        VALUES ($1, $2, $3, 'consumo', $4::float8::numeric,
                $5, $6,
                $7, $8, $9, $10,
                $11, $12, $13)
        ON CONFLICT DO NOTHING
        RETURNING id, clave_idempotencia, lote_id, programacion_id, tipo,
                  peso::float8        AS "peso!",
                  peso_efecto::float8 AS "peso_efecto!",
                  operario_id, registrado_por, motivo, observaciones,
                  corrige_movimiento_id,
                  medido_en, recibido_concentrador_en, concentrador_id, bascula_id,
                  sesion_arranque, transaccion, uid_tarjeta,
                  version, creado_en, actualizado_en
        "#,
        datos.clave_idempotencia,
        datos.lote_id,
        datos.programacion_id,
        datos.peso,
        datos.operario_id,
        registrado_por,
        datos.medido_en,
        datos.recibido_concentrador_en,
        datos.concentrador_id,
        datos.bascula_id,
        datos.sesion_arranque,
        datos.transaccion,
        datos.uid_tarjeta,
    )
    .fetch_optional(&mut *conexion)
    .await
    .map_err(map_conflicto_movimiento)?;

    if let Some(movimiento) = insertado {
        return Ok(AltaMovimiento {
            movimiento,
            creado: true,
        });
    }

    // No insertó: es un pesaje que ya estaba. Se busca por las dos claves
    // porque no se sabe cuál de los dos UNIQUE lo frenó.
    let existente = sqlx::query_as!(
        MovimientosMateriaPrima,
        r#"
        SELECT id, clave_idempotencia, lote_id, programacion_id, tipo,
               peso::float8        AS "peso!",
               peso_efecto::float8 AS "peso_efecto!",
               operario_id, registrado_por, motivo, observaciones,
               corrige_movimiento_id,
               medido_en, recibido_concentrador_en, concentrador_id, bascula_id,
               sesion_arranque, transaccion, uid_tarjeta,
               version, creado_en, actualizado_en
        FROM movimientos_materia_prima
        WHERE clave_idempotencia = $1
           OR (bascula_id = $2 AND sesion_arranque = $3 AND transaccion = $4)
        LIMIT 1
        "#,
        datos.clave_idempotencia,
        datos.bascula_id,
        datos.sesion_arranque,
        datos.transaccion,
    )
    .fetch_one(&mut *conexion)
    .await
    // RowNotFound acá sería que la fila que frenó el INSERT desapareció entre
    // las dos consultas, y un movimiento no se borra: es un 500, no un 404.
    .map_err(map_conflicto_movimiento)?;

    Ok(AltaMovimiento {
        movimiento: existente,
        creado: false,
    })
}
