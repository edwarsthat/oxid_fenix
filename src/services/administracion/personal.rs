use sqlx::PgPool;

use crate::{
    models::administracion::personal::{Empleado, PersonalFiltros},
    services::error::ServiceError,
};

pub async fn get_personal(
    pool: &PgPool,
    filtros: PersonalFiltros,
) -> Result<Vec<Empleado>, ServiceError> {
    let retiro_desde = filtros.retiro.and_then(|r| r.desde());
    let retiro_hasta = filtros.retiro.and_then(|r| r.hasta());

    let personal = sqlx::query_as!(
        Empleado,
        r#"
        SELECT id, codigo, tipo_documento, documento, nombre, apellido,
               fecha_nacimiento, telefono, cargo_id, fecha_ingreso,
               fecha_retiro, activo, creado_en, actualizado_en
        FROM personal
        WHERE activo = $1
          AND ($2::uuid IS NULL OR cargo_id = $2)
          AND ($3::text IS NULL
               OR nombre    ILIKE '%' || $3 || '%'
               OR apellido  ILIKE '%' || $3 || '%'
               OR documento ILIKE $3 || '%')
          AND ($4::date IS NULL OR fecha_retiro >= $4)
          AND ($5::date IS NULL OR fecha_retiro <= $5)
        ORDER BY creado_en DESC, id DESC
        LIMIT $6
        "#,
        filtros.activo,
        filtros.cargo_id,
        filtros.busqueda.as_deref(),
        retiro_desde,
        retiro_hasta,
        filtros.limite,
    )
    .fetch_all(pool)
    .await?;

    Ok(personal)
}
