use sqlx::PgPool;

use crate::{
    models::inventarios::materia_prima::inventario_materia_prima::InventarioMateriaPrima,
    services::error::ServiceError,
};

/// Lo que hay en patio ahora mismo: un renglón por lote abierto, con sus kilos.
///

pub async fn read_inventario_materia_prima(
    pool: &PgPool,
) -> Result<Vec<InventarioMateriaPrima>, ServiceError> {
    // El WHERE es literalmente la condición de `idx_ingresos_mp_patio`
    // (`materia_prima_id, llegada_en` WHERE cerrado_en IS NULL AND anulado_en
    // IS NULL). Eso importa: ese índice parcial solo contiene los lotes
    // abiertos, así que esta consulta lee decenas de filas por más que la tabla
    // llegue a millones, y el ORDER BY sale del mismo índice. Si alguien le
    // cambia una condición al WHERE, el índice deja de aplicar y la consulta
    // pasa a recorrer toda la historia sin que nada falle a la vista.
    //
    // `peso_devuelto` no aparece por ningún lado: es lo que se rechazó en
    // portería y se devolvió en el mismo camión, nunca entró al patio y ya está
    // fuera de `peso_ingreso`. Restarlo sería restarlo dos veces.
    let inventario = sqlx::query_as!(
        InventarioMateriaPrima,
        r#"
        SELECT i.id AS lote_id,
               i.codigo,
               i.materia_prima_id,
               mp.nombre AS materia_prima,
               p.nombre  AS predio,
               pr.nombre AS proveedor,
               i.llegada_en,
               i.peso_ingreso::float8              AS "peso_ingreso!",
               (-m.efecto)::float8                 AS "consumido!",
               (i.peso_ingreso + m.efecto)::float8 AS "saldo!"
        FROM ingresos_materia_prima i
        JOIN materias_primas mp ON mp.id = i.materia_prima_id
        JOIN predios p          ON p.id  = i.predio_id
        -- El proveedor cuelga del predio y no del lote: `predios.proveedor_id`
        -- es NOT NULL, así que el JOIN interno no puede perder filas.
        JOIN proveedores pr     ON pr.id = p.proveedor_id
        -- CROSS y no LEFT: el subquery es un agregado, así que siempre devuelve
        -- exactamente una fila. Un lote sin movimientos da 0 por el COALESCE, no
        -- NULL, y por eso el saldo de un lote recién ingresado es su peso de
        -- ingreso completo en vez de NULL.
        --
        -- LATERAL y no un GROUP BY sobre todo el SELECT: así cada lote resuelve
        -- su suma contra `idx_movimientos_mp_lote`, que trae `peso_efecto` en el
        -- INCLUDE y no tiene que tocar la tabla de movimientos.
        CROSS JOIN LATERAL (
            SELECT COALESCE(SUM(mv.peso_efecto), 0) AS efecto
            FROM movimientos_materia_prima mv
            WHERE mv.lote_id = i.id
        ) m
        -- `cerrado_en` es un atajo, no la verdad: el saldo real es la resta de
        -- abajo. Se filtra por la marca igual, porque es lo que permite no
        -- calcularle el saldo a todos los lotes de la historia solo para
        -- descartarlos. Si algún día la marca y la resta no coinciden, un lote
        -- cerrado con saldo no sale acá — manda la resta, pero eso se arregla
        -- revisando el cierre, no ensanchando esta consulta.
        WHERE i.cerrado_en IS NULL
          AND i.anulado_en IS NULL
        -- FEFO: primero el que lleva más tiempo esperando, que con
        -- `horas_maximas_espera` de por medio es el urgente. El orden lo pone el
        -- servidor y no el cliente porque es la regla del negocio, no una
        -- preferencia de la pantalla.
        --
        -- `i.id` desempata: dos lotes pueden compartir `llegada_en` si el
        -- cliente la mandó a mano, y sin desempate el orden entre esos dos
        -- cambiaría entre consultas.
        ORDER BY i.llegada_en, i.id
        "#
    )
    .fetch_all(pool)
    .await?;

    Ok(inventario)
}
