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
    .bind("edwarsthat@gmail.com")
    .bind("admin")
    .bind(&hash)
    .bind(cargo_id)
    .bind(true)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        println!("[seed::usuarios] 'admin@celifrut.com' ya existía.");
    } else {
        println!("[seed::usuarios] admin creado");
    }

    Ok(())
}
