use sqlx::PgPool;
use uuid::Uuid;

use crate::seeds::error::SeedError;

pub async fn seed(pool: &PgPool) -> Result<(), SeedError> {
    let res = sqlx::query(
        r#"
        INSERT INTO cargos (nombre, descripcion)
        VALUES ($1, $2)
        ON CONFLICT (nombre) DO NOTHING
        "#,
    )
    .bind("admin")
    .bind("Administrador del sistema, Maximo rol del negocio.")
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        println!("[seed::cargos] 'admin' ya existe")
    } else {
        println!("[seed::cargos] 'admin' creado")
    }

    let res = sqlx::query(
        r#"
        INSERT INTO cargos (nombre, descripcion)
        VALUES ($1, $2)
        ON CONFLICT (nombre) DO NOTHING
        "#,
    )
    .bind("sistema")
    .bind("Cargo de los usuarios de sistema (procesos automáticos). Sin permisos.")
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        println!("[seed::cargos] 'sistema' ya existe")
    } else {
        println!("[seed::cargos] 'sistema' creado")
    }

    Ok(())
}

pub async fn id_por_nombre(pool: &PgPool, nombre: &str) -> Result<Uuid, SeedError> {
    let id: Uuid = sqlx::query_scalar("SELECT id FROM cargos WHERE nombre = $1")
        .bind(nombre)
        .fetch_one(pool)
        .await?;
    Ok(id)
}
