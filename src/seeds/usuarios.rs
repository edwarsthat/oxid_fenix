use sqlx::PgPool;

use crate::security::password::hashear;
use crate::seeds::cargos;
use crate::seeds::error::SeedError;

pub async fn seed(pool: &PgPool) -> Result<(), SeedError> {
    let cargo_id = cargos::id_por_nombre(pool, "admin").await?;
    let password = std::env::var("ADMIN_PASSWORD").unwrap_or_else(|_| "admin1234".to_string());
    let hash = hashear(&password)?;

    let res = sqlx::query(
        r#"
        INSERT INTO usuarios (nombre, apellido, email, usuario, password_hash, cargo_id, debe_cambiar_password)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        ON CONFLICT (usuario) DO NOTHING
        "#,
    )
    .bind("Admin")
    .bind("Principal")
    .bind("admin@agrofenix.com")
    .bind("admin")
    .bind(&hash)
    .bind(cargo_id)
    .bind(true)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        println!("[seed::usuarios] 'admin@agrofenix.com' ya existía.");
    } else {
        println!("[seed::usuarios] admin creado");
    }

    // Usuario de sistema de las básculas: es el `registrado_por` de los
    // pesajes. No inicia sesión: inactivo y con un hash '!' que ningún
    // password verifica.
    let cargo_sistema_id = cargos::id_por_nombre(pool, "sistema").await?;

    let res = sqlx::query(
        r#"
        INSERT INTO usuarios (nombre, apellido, email, usuario, password_hash, cargo_id, activo, debe_cambiar_password)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (usuario) DO NOTHING
        "#,
    )
    .bind("Sistema")
    .bind("Básculas")
    .bind("sistema.basculas@agrofenix.com")
    .bind("sistema.basculas")
    .bind("!")
    .bind(cargo_sistema_id)
    .bind(false)
    .bind(false)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        println!("[seed::usuarios] 'sistema.basculas' ya existía.");
    } else {
        println!("[seed::usuarios] sistema.basculas creado");
    }

    Ok(())
}
