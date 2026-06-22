use sqlx::PgPool;

use crate::seeds::cargos;
use crate::seeds::error::SeedError;

pub async fn seed(pool: &PgPool) -> Result<(), SeedError> {
    let cargo_id = cargos::id_por_nombre(pool, "admin").await?;

    let res = sqlx::query(
        r#"
        INSERT INTO cargos_permisos (cargo_id, permiso_id)
        SELECT $1, p.id
        FROM permisos p
        ON CONFLICT (cargo_id, permiso_id) DO NOTHING
        "#,
    )
    .bind(cargo_id)
    .execute(pool)
    .await?;

    println!(
        "[seed::cargos_permisos] {} permisos asignados a 'admin'",
        res.rows_affected()
    );
    Ok(())
}
