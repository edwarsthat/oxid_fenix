// src/seeds/permisos.rs
use sqlx::PgPool;

use crate::seeds::error::SeedError;

/// Acciones CRUD estándar que aplican a (casi) todos los módulos.
const CRUD: &[&str] = &["add", "read", "update", "delete"];

/// Módulos del sistema y las acciones que realmente existen en cada uno.
/// Agregar uno nuevo = una línea aquí. Si el módulo no expone todo el CRUD,
/// se listan solo sus acciones reales para no sembrar permisos muertos.
const MODULOS: &[(&str, &[&str])] = &[
    //administracio
    ("usuarios", CRUD),
    ("cargos", CRUD),
    ("permisos", CRUD),
    // Las sesiones no se crean ni se editan desde la administración:
    // solo se listan y se revocan (cerrar sesión de un usuario).
    ("sesiones", &["read", "delete"]),
    //talento humano
    ("cargos_personal", CRUD),
    ("personal", CRUD),
    //inventarios
    ("llaves_nfc", CRUD),
    //proveedores
    ("proveedores", CRUD),
    // Por ahora solo el alta: el resto del CRUD de predios no tiene endpoint.
    ("predios", &["add"]),
    ("predios", CRUD),
    //catalogos
    ("materias_primas", &["read"]),
];

/// Siembra el producto módulos × acciones. Idempotente.
pub async fn seed(pool: &PgPool) -> Result<(), SeedError> {
    for (modulo, acciones) in MODULOS {
        for accion in *acciones {
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
    println!(
        "[seed::permisos] permisos sembrados ({} módulos)",
        MODULOS.len()
    );
    Ok(())
}
