use sqlx::PgPool;

use crate::seeds::error::SeedError;

pub async fn seed(pool: &PgPool) -> Result<(), SeedError> {
    let res = sqlx::query(
        r#"
        INSERT INTO cargos_personal (nombre, tipo_contrato)
        VALUES ($1, $2)
        ON CONFLICT (nombre) DO NOTHING
        "#,
    )
    .bind("pelador")
    .bind("destajo")
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        println!("[seed::cargos_personal] 'pelador' ya existe")
    } else {
        println!("[seed::cargos_personal] 'pelador' creado")
    }

    Ok(())
}
