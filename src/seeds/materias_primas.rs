// src/seeds/materias_primas.rs
use sqlx::PgPool;

use crate::seeds::error::SeedError;

/// Una fila del catálogo. Va como struct y no como tupla porque son seis
/// campos y tres de ellos son porcentajes: en tupla no se distingue cuál es
/// cuál al leer la lista.
struct MateriaPrimaSeed {
    codigo: &'static str,
    nombre: &'static str,
    /// NULL = sin límite de espera.
    horas_maximas_espera: Option<i32>,
    rendimiento_esperado_pct: Option<f64>,
    rendimiento_min_pct: Option<f64>,
    rendimiento_max_pct: Option<f64>,
}

/// El catálogo completo. Esta lista es la fuente de verdad: no hay endpoint de
/// escritura, así que agregar o ajustar una materia prima es editar acá y
/// volver a correr `cargo run --bin seeds`.
///
/// Los porcentajes son provisionales y hay que confirmarlos contra lo que
/// mide producción antes de colgarles alertas de proceso fuera de rango.
const MATERIAS_PRIMAS: &[MateriaPrimaSeed] = &[
    MateriaPrimaSeed {
        codigo: "PLA",
        nombre: "Plátano",
        // Aguanta días, por eso no se le pone tope de espera.
        horas_maximas_espera: None,
        rendimiento_esperado_pct: Some(62.0),
        rendimiento_min_pct: Some(55.0),
        rendimiento_max_pct: Some(70.0),
    },
    MateriaPrimaSeed {
        codigo: "YUC",
        nombre: "Yuca",
        // Se deteriora rápido: pasadas ~48h ya no sirve para proceso.
        horas_maximas_espera: Some(48),
        rendimiento_esperado_pct: Some(70.0),
        rendimiento_min_pct: Some(62.0),
        rendimiento_max_pct: Some(78.0),
    },
];

/// Siembra el catálogo. Idempotente: correrlo dos veces seguidas no cambia
/// nada.
pub async fn seed(pool: &PgPool) -> Result<(), SeedError> {
    let mut cambios = 0;

    for materia_prima in MATERIAS_PRIMAS {
        // A diferencia de los otros seeds esto es un UPDATE y no un DO
        // NOTHING: como la lista de arriba es la fuente de verdad, editar un
        // rendimiento y correr el seed tiene que reflejarse en la tabla; con
        // DO NOTHING el cambio se perdería en silencio.
        //
        // El WHERE evita el UPDATE cuando la fila ya está igual, para que una
        // corrida sin cambios no dispare el trigger y suba `version` y
        // `actualizado_en` de gratis.
        //
        // `activo` queda fuera del SET a propósito: si alguien dio de baja una
        // materia prima, el seed no la revive.
        //
        // Los porcentajes van con cast explícito porque las columnas son
        // NUMERIC y acá se enlazan como float8.
        let res = sqlx::query(
            r#"
            INSERT INTO materias_primas (codigo, nombre, horas_maximas_espera,
                                         rendimiento_esperado_pct,
                                         rendimiento_min_pct, rendimiento_max_pct)
            VALUES ($1, $2, $3, $4::float8::numeric, $5::float8::numeric, $6::float8::numeric)
            ON CONFLICT (codigo) DO UPDATE
            SET nombre                   = EXCLUDED.nombre,
                horas_maximas_espera     = EXCLUDED.horas_maximas_espera,
                rendimiento_esperado_pct = EXCLUDED.rendimiento_esperado_pct,
                rendimiento_min_pct      = EXCLUDED.rendimiento_min_pct,
                rendimiento_max_pct      = EXCLUDED.rendimiento_max_pct
            WHERE (materias_primas.nombre, materias_primas.horas_maximas_espera,
                   materias_primas.rendimiento_esperado_pct,
                   materias_primas.rendimiento_min_pct, materias_primas.rendimiento_max_pct)
                  IS DISTINCT FROM
                  (EXCLUDED.nombre, EXCLUDED.horas_maximas_espera,
                   EXCLUDED.rendimiento_esperado_pct,
                   EXCLUDED.rendimiento_min_pct, EXCLUDED.rendimiento_max_pct)
            "#,
        )
        .bind(materia_prima.codigo)
        .bind(materia_prima.nombre)
        .bind(materia_prima.horas_maximas_espera)
        .bind(materia_prima.rendimiento_esperado_pct)
        .bind(materia_prima.rendimiento_min_pct)
        .bind(materia_prima.rendimiento_max_pct)
        .execute(pool)
        .await?;

        cambios += res.rows_affected();
    }

    println!(
        "[seed::materias_primas] {} materias primas sembradas ({cambios} con cambios)",
        MATERIAS_PRIMAS.len()
    );
    Ok(())
}
