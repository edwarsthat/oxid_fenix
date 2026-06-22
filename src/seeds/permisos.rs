// src/seeds/permisos.rs
use sqlx::PgPool;

use crate::seeds::error::SeedError;

/// Acciones CRUD estándar que aplican a (casi) todos los módulos.
const ACCIONES: &[&str] = &["add", "read", "write", "update", "delete"];

/// Módulos del sistema. Agregar uno nuevo = una línea aquí.
const MODULOS: &[&str] = &[
    "usuarios",
    // "productos",
    // "cargos",
];

/// Siembra el producto módulos × acciones. Idempotente.
pub async fn seed(pool: &PgPool) -> Result<(), SeedError> {
    for modulo in MODULOS {
        for accion in ACCIONES {
            let nombre = format!("{modulo}:{accion}");
            let descripcion = format!("Permite {accion} en el módulo {modulo}");

            sqlx::query(
                r#"
                INSERT INTO permisos (nombre, descripcion)
                VALUES ($1, $2)
                ON CONFLICT (nombre) DO NOTHING
                "#,
            )
            .bind(&nombre)
            .bind(&descripcion)
            .execute(pool)
            .await?;
        }
    }
    println!("[seed::permisos] permisos sembrados ({} módulos)", MODULOS.len());
    Ok(())
}
