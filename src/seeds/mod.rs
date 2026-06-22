pub mod cargos;
pub mod cargos_permisos;
pub mod error;
pub mod permisos;
pub mod usuarios;

use sqlx::PgPool;

use crate::seeds::error::SeedError;

pub async fn run_all(pool: &PgPool) -> Result<(), SeedError> {
    println!("[seed] iniciando siembra de datos...");

    cargos::seed(pool).await?;
    permisos::seed(pool).await?;
    cargos_permisos::seed(pool).await?;
    usuarios::seed(pool).await?;

    println!("[seed] siembra completada.");
    Ok(())
}
