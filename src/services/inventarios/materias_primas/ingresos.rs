use uuid::Uuid;

use crate::{
    models::inventarios::materia_prima::ingresos::{AltaIngreso, Ingreso, IngresoNuevo},
    services::error::ServiceError,
};

/// Traduce los errores de integridad de `ingresos_materia_prima` a errores de
/// negocio; si no, mandar un predio que ya no existe saldría como 500.
fn map_conflicto_ingreso(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            // El UNIQUE de la clave de idempotencia no debería llegar acá: el
            // INSERT lo absorbe con ON CONFLICT. Queda el del codigo, que solo
            // pasa si la secuencia quedó desincronizada.
            Some("23505") => {
                let mensaje = match db_err.constraint() {
                    Some("ingresos_materia_prima_codigo_key") => {
                        "ya existe un ingreso con ese código"
                    }
                    _ => "el ingreso choca con uno ya registrado",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            // Las FK son tres y dos de ellas las manda el cliente, así que el
            // mensaje tiene que decir cuál id está mal. `registrado_por` sale
            // de la sesión: si falla, el usuario se borró con la sesión viva.
            Some("23503") => {
                let mensaje = match db_err.constraint() {
                    Some("ingresos_materia_prima_materia_prima_id_fkey") => {
                        "la materia prima indicada no existe"
                    }
                    Some("ingresos_materia_prima_registrado_por_fkey") => {
                        "el usuario que registra el ingreso ya no existe"
                    }
                    _ => "el predio indicado no existe",
                };
                return ServiceError::NotFound(mensaje.into());
            }
            // `IngresoAddPayload` ya valida estas mismas reglas, así que llegar
            // acá significa que el dato entró sin pasar por `validar()`. Igual
            // conviene decir qué columna y no un 23514 pelado.
            //
            // El check de descargue contra `llegada_en` sí puede caer acá por
            // la vía normal: cuando el cliente no manda la llegada, el payload
            // no tiene contra qué comparar el inicio y la regla la termina
            // verificando la tabla contra su DEFAULT.
            Some("23514") => {
                let mensaje = match db_err.constraint() {
                    Some("ingresos_materia_prima_placa_check") => {
                        "la placa debe tener entre 5 y 7 caracteres alfanuméricos"
                    }
                    Some("ingresos_materia_prima_peso_ingreso_check") => {
                        "el peso de ingreso debe ser mayor que cero"
                    }
                    Some("ingresos_materia_prima_peso_devuelto_check") => {
                        "el peso devuelto no puede ser negativo"
                    }
                    Some("ingresos_materia_prima_inicio_descargue_check") => {
                        "el descargue no puede empezar antes de que llegue el camión"
                    }
                    Some("ingresos_materia_prima_fin_descargue_check") => {
                        "el fin de descargue no puede ir antes del inicio"
                    }
                    _ => "el ingreso no cumple una regla de la tabla",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

/// Alta de un lote de materia prima.
///
/// `registrado_por` va aparte de `datos` a propósito: sale de la sesión
/// (`ctx.user_id`) y no del payload, así que no puede viajar dentro de algo que
/// armó el cliente.
///
/// Es idempotente por `clave_idempotencia`: si el mismo alta llega dos veces —
/// la tablet de báscula reintentando porque se le cayó la señal justo después
/// del INSERT — la segunda devuelve el lote que ya se creó, con `creado: false`,
/// en vez de un lote gemelo o un 409 que el usuario no sabría qué hacer.
pub async fn add_ingreso_lote_materia_prima(
    conexion: &mut sqlx::PgConnection,
    datos: IngresoNuevo,
    registrado_por: Uuid,
) -> Result<AltaIngreso, ServiceError> {
    // `codigo`, `cerrado_en`, la anulación, `version` y las fechas de auditoría
    // se omiten a propósito: el consecutivo sale de la secuencia, el cierre lo
    // pone el trigger del libro de movimientos y anular es otra operación.
    //
    // `fecha_ingreso` y `llegada_en` entran con COALESCE contra el DEFAULT de
    // la tabla: cuando el cliente no las manda, la hora la pone el reloj del
    // servidor de base y no el de este proceso.
    //
    // Los pesos van y vuelven casteados: la columna es NUMERIC y el driver no
    // trae tipo decimal habilitado, así que el parámetro entra como float8. El
    // `!` del RETURNING es porque el cast le borra a sqlx el NOT NULL de la
    // columna y sin él los tipos no cuadran con el struct.
    let insertado = sqlx::query_as!(
        Ingreso,
        r#"
        INSERT INTO ingresos_materia_prima
            (clave_idempotencia, predio_id, materia_prima_id, placa,
             numero_remision, numero_tiquete_bascula,
             fecha_ingreso, llegada_en, inicio_descargue_en, fin_descargue_en,
             peso_ingreso, peso_devuelto, observaciones, registrado_por)
        VALUES ($1, $2, $3, $4, $5, $6,
                COALESCE($7::date, CURRENT_DATE),
                COALESCE($8::timestamptz, NOW()),
                $9, $10,
                $11::float8::numeric, $12::float8::numeric,
                $13, $14)
        -- El corazón de la idempotencia. DO NOTHING y no DO UPDATE: un ingreso
        -- no se corrige por la vía del reintento, y pisar la fila con lo que
        -- mandó el segundo intento cambiaría un lote ya registrado.
        ON CONFLICT (clave_idempotencia) DO NOTHING
        RETURNING id, codigo, clave_idempotencia, predio_id, materia_prima_id,
                  placa, numero_remision, numero_tiquete_bascula,
                  fecha_ingreso, llegada_en, inicio_descargue_en, fin_descargue_en,
                  peso_ingreso::float8  AS "peso_ingreso!",
                  peso_devuelto::float8 AS "peso_devuelto!",
                  cerrado_en, observaciones, registrado_por,
                  anulado_en, anulado_por, motivo_anulacion,
                  version, creado_en, actualizado_en
        "#,
        datos.clave_idempotencia,
        datos.predio_id,
        datos.materia_prima_id,
        datos.placa,
        datos.numero_remision,
        datos.numero_tiquete_bascula,
        datos.fecha_ingreso,
        datos.llegada_en,
        datos.inicio_descargue_en,
        datos.fin_descargue_en,
        datos.peso_ingreso,
        datos.peso_devuelto,
        datos.observaciones,
        registrado_por,
    )
    .fetch_optional(&mut *conexion)
    .await
    .map_err(map_conflicto_ingreso)?;

    if let Some(ingreso) = insertado {
        return Ok(AltaIngreso {
            ingreso,
            creado: true,
        });
    }

    // No insertó: la clave ya estaba, o sea que este es el reintento de un alta
    // que sí funcionó. Se devuelve el lote de la primera, que es lo que el
    // cliente estaba esperando cuando se le cayó la señal.
    let existente = sqlx::query_as!(
        Ingreso,
        r#"
        SELECT id, codigo, clave_idempotencia, predio_id, materia_prima_id,
               placa, numero_remision, numero_tiquete_bascula,
               fecha_ingreso, llegada_en, inicio_descargue_en, fin_descargue_en,
               peso_ingreso::float8  AS "peso_ingreso!",
               peso_devuelto::float8 AS "peso_devuelto!",
               cerrado_en, observaciones, registrado_por,
               anulado_en, anulado_por, motivo_anulacion,
               version, creado_en, actualizado_en
        FROM ingresos_materia_prima
        WHERE clave_idempotencia = $1
        "#,
        datos.clave_idempotencia,
    )
    .fetch_one(&mut *conexion)
    .await
    // RowNotFound acá sería que la fila que bloqueó el INSERT desapareció entre
    // las dos consultas, y un ingreso no se borra: eso es un 500, no un 404.
    .map_err(map_conflicto_ingreso)?;

    Ok(AltaIngreso {
        ingreso: existente,
        creado: false,
    })
}
