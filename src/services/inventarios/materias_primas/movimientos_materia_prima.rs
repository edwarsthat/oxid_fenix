use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    models::inventarios::materia_prima::movimientos_materia_prima::{
        AltaMovimiento, ConsumoBasculaNuevo, CursorPesaje, MovimientosMateriaPrima,
        PaginaPesajes, PesajeListado, PesajesFiltros, TotalPesajesOperario,
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

/// Si ya hay un movimiento con esa `clave_idempotencia` (el `id` de la Pi).
///
/// El endpoint de pesajes la mira antes de resolver báscula, lote y operario:
/// un reintento no tiene que depender de que eso se siga resolviendo igual
/// (una báscula desactivada después del primer envío lo haría salir 422 y la
/// Pi lo daría por rechazado estando guardado). No reemplaza al `ON CONFLICT`
/// de `add_consumo_bascula`: ese sigue atrapando la terna del ESP32 y dos
/// envíos simultáneos que pasen los dos por acá antes de insertar.
pub async fn existe_clave_idempotencia<'e, E>(
    executor: E,
    clave: Uuid,
) -> Result<bool, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    let existe = sqlx::query_scalar!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM movimientos_materia_prima
            WHERE clave_idempotencia = $1
        ) AS "existe!"
        "#,
        clave,
    )
    .fetch_one(executor)
    .await?;

    Ok(existe)
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

/// Listado de pesajes de báscula (consumos que llegaron desde un
/// concentrador), paginado.
///
/// La paginación es keyset sobre `(medido_en, id)` y no OFFSET: los pesajes
/// entran en vivo mientras alguien mira la pantalla, y con OFFSET cada pesaje
/// nuevo corre todas las páginas un lugar, así que la página 2 repite la
/// última fila de la 1 (o se salta una si entró tarde un pesaje viejo por la
/// cola de la Pi). Con el cursor la página siguiente arranca justo después de
/// la última fila vista, entre lo que pase en el medio.
///
/// Mismo patrón de filtros que `get_llaves_nfc`: SQL estático y cada filtro
/// se apaga con `$n IS NULL`.
pub async fn get_pesajes_bascula(
    pool: &PgPool,
    filtros: PesajesFiltros,
) -> Result<PaginaPesajes, ServiceError> {
    let limite = filtros.limite;
    let cursor_medido_en = filtros.cursor.map(|c| c.medido_en);
    let cursor_id = filtros.cursor.map(|c| c.id);

    // Por el CHECK de origen completo, en una fila con `bascula_id` los siete
    // campos de origen van llenos, y por ser consumo también `programacion_id`
    // y `operario_id`: de ahí los `!`, que sqlx no puede deducir solo.
    let mut pesajes = sqlx::query_as!(
        PesajeListado,
        r#"
        SELECT m.id, m.lote_id,
               m.programacion_id          AS "programacion_id!",
               m.operario_id              AS "operario_id!",
               m.bascula_id               AS "bascula_id!",
               m.peso::float8             AS "peso!",
               m.medido_en                AS "medido_en!",
               m.recibido_concentrador_en AS "recibido_concentrador_en!",
               m.creado_en,
               m.sesion_arranque          AS "sesion_arranque!",
               m.transaccion              AS "transaccion!",
               m.uid_tarjeta              AS "uid_tarjeta!",
               -- LEFT JOIN, pero el operario siempre está: la FK y el CHECK
               -- de consumo lo garantizan. El LEFT es para que una fila nunca
               -- desaparezca del listado del destajo por un join.
               p.codigo   AS "operario_codigo!",
               p.nombre   AS "operario_nombre!",
               p.apellido AS "operario_apellido!",
               ln.codigo  AS "llave_codigo?",
               b.codigo   AS bascula_codigo,
               i.codigo   AS lote_codigo,
               mp.nombre  AS materia_prima_nombre,
               cr.corregido AS "corregido!"
        FROM movimientos_materia_prima m
        LEFT JOIN personal p ON p.id = m.operario_id
        -- Sin FK entre `uid_tarjeta` y `llaves_nfc`: una tarjeta dada de baja
        -- y borrada, o cualquier uid que ya no esté registrado, no tiene que
        -- tumbar el pesaje. `llaves_nfc.uid` es UNIQUE, así que no duplica.
        LEFT JOIN llaves_nfc ln ON ln.uid = m.uid_tarjeta
        JOIN basculas b ON b.id = m.bascula_id
        JOIN ingresos_materia_prima i ON i.id = m.lote_id
        JOIN materias_primas mp ON mp.id = i.materia_prima_id
        -- En un LATERAL y no repetido en el SELECT y el WHERE: es el mismo
        -- EXISTS para las dos cosas. Lo resuelve idx_movimientos_mp_correccion.
        CROSS JOIN LATERAL (
            SELECT EXISTS (
                SELECT 1 FROM movimientos_materia_prima c
                WHERE c.corrige_movimiento_id = m.id
            ) AS corregido
        ) cr
        WHERE m.bascula_id IS NOT NULL
          -- Siempre `medido_en` y nunca `creado_en`: un pesaje que la Pi tuvo
          -- encolado llega horas después, y por `creado_en` caería en otro
          -- turno o en otro día del destajo.
          AND m.medido_en >= $1
          AND m.medido_en <  $2
          AND ($3::uuid IS NULL OR m.operario_id = $3)
          AND ($4::uuid IS NULL OR ln.id = $4)
          AND ($5::uuid IS NULL OR m.lote_id = $5)
          AND ($6::uuid IS NULL OR m.programacion_id = $6)
          AND ($7::uuid IS NULL OR m.bascula_id = $7)
          -- Por el uid que leyó el lector y no por la llave: encuentra también
          -- los pesajes de una tarjeta que no está (o ya no está) registrada.
          AND ($8::text IS NULL OR m.uid_tarjeta = $8)
          AND ($9::bool IS NULL OR cr.corregido = $9)
          -- Comparación de filas: estrictamente antes del cursor en el mismo
          -- orden del ORDER BY, con el id desempatando el mismo instante.
          AND ($10::timestamptz IS NULL OR (m.medido_en, m.id) < ($10, $11::uuid))
        ORDER BY m.medido_en DESC, m.id DESC
        LIMIT $12
        "#,
        filtros.desde,
        filtros.hasta,
        filtros.operario_id,
        filtros.llave_id,
        filtros.lote_id,
        filtros.programacion_id,
        filtros.bascula_id,
        filtros.uid,
        filtros.corregido,
        cursor_medido_en,
        cursor_id,
        // Una fila de más para saber si hay página siguiente sin un COUNT.
        limite + 1,
    )
    .fetch_all(pool)
    .await?;

    // `limitar` deja el límite en [1, LIMITE_MAXIMO], así que el cast no pierde
    // nada.
    let siguiente_cursor = if pesajes.len() > limite as usize {
        pesajes.truncate(limite as usize);
        // El cursor es la última fila que SÍ se devuelve, no la sobrante: la
        // siguiente página la pide con `<`, así que la sobrante es su primera.
        pesajes.last().map(|ultimo| CursorPesaje {
            medido_en: ultimo.medido_en,
            id: ultimo.id,
        })
    } else {
        None
    };

    Ok(PaginaPesajes {
        pesajes,
        siguiente_cursor,
    })
}

/// Totales por operario con los mismos filtros del listado, sin cursor ni
/// límite.
///
/// Van sobre todo el filtro y no sobre la página porque es el número del
/// destajo: sumarlo en el front con la página que tenga cargada da mal en
/// cuanto el día pasa de un límite de filas.
///
/// Los kg son brutos: no restan los ajustes que corrigen estos pesajes.
/// `cantidad_corregidos` está para que la pantalla señale al operario que
/// tiene pesajes corregidos y alguien revise antes de pagar.
///
/// No lleva los joins de báscula, lote y materia prima del listado: ninguna
/// columna de acá sale de ellos, y como son FKs NOT NULL no quitan filas.
pub async fn get_totales_pesajes_bascula(
    pool: &PgPool,
    filtros: &PesajesFiltros,
) -> Result<Vec<TotalPesajesOperario>, ServiceError> {
    let totales = sqlx::query_as!(
        TotalPesajesOperario,
        r#"
        SELECT m.operario_id AS "operario_id!",
               p.codigo      AS "operario_codigo!",
               p.nombre      AS "operario_nombre!",
               p.apellido    AS "operario_apellido!",
               COUNT(*)      AS "cantidad!",
               -- SUM de NUMERIC es exacto; el float8 es solo para el struct.
               SUM(m.peso)::float8 AS "kg!",
               COUNT(*) FILTER (WHERE cr.corregido) AS "cantidad_corregidos!"
        FROM movimientos_materia_prima m
        LEFT JOIN personal p ON p.id = m.operario_id
        LEFT JOIN llaves_nfc ln ON ln.uid = m.uid_tarjeta
        CROSS JOIN LATERAL (
            SELECT EXISTS (
                SELECT 1 FROM movimientos_materia_prima c
                WHERE c.corrige_movimiento_id = m.id
            ) AS corregido
        ) cr
        -- El mismo WHERE de `get_pesajes_bascula` sin el cursor: si cambia
        -- uno, cambia el otro, o los totales dejan de cuadrar con el listado.
        WHERE m.bascula_id IS NOT NULL
          AND m.medido_en >= $1
          AND m.medido_en <  $2
          AND ($3::uuid IS NULL OR m.operario_id = $3)
          AND ($4::uuid IS NULL OR ln.id = $4)
          AND ($5::uuid IS NULL OR m.lote_id = $5)
          AND ($6::uuid IS NULL OR m.programacion_id = $6)
          AND ($7::uuid IS NULL OR m.bascula_id = $7)
          AND ($8::text IS NULL OR m.uid_tarjeta = $8)
          AND ($9::bool IS NULL OR cr.corregido = $9)
        -- `p.id` es la PK, así que agrupar por él deja usar el resto de sus
        -- columnas en el SELECT sin listarlas.
        GROUP BY m.operario_id, p.id
        -- El código desempata para que el orden no salte entre recargas.
        ORDER BY SUM(m.peso) DESC, p.codigo
        "#,
        filtros.desde,
        filtros.hasta,
        filtros.operario_id,
        filtros.llave_id,
        filtros.lote_id,
        filtros.programacion_id,
        filtros.bascula_id,
        filtros.uid.as_deref(),
        filtros.corregido,
    )
    .fetch_all(pool)
    .await?;

    Ok(totales)
}
