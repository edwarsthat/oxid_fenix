use sqlx::PgPool;

use crate::{
    models::proveedores::proveedores::{Proveedor, ProveedorNuevo, ProveedoresFiltros},
    services::error::ServiceError,
};

/// Traduce los errores de integridad de la tabla `proveedores` a errores de
/// negocio; si no, cargar dos veces el mismo NIT saldría como 500.
fn map_conflicto_proveedor(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            // Hay dos UNIQUE en la tabla y el mensaje tiene que decir cuál
            // falló: el del documento lo provoca el usuario y lo puede
            // corregir; el del codigo solo pasa si la secuencia quedó
            // desincronizada.
            Some("23505") => {
                let mensaje = match db_err.constraint() {
                    Some("proveedores_codigo_key") => "ya existe un proveedor con ese código",
                    _ => "ya existe un proveedor con ese tipo y número de documento",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            // Los tres CHECK de la tabla. El payload ya los valida contra las
            // mismas listas, así que llegar acá significa que alguien insertó
            // sin pasar por `ProveedorAddPayload`: igual conviene un mensaje
            // que diga qué columna, y no un 23514 pelado.
            Some("23514") => {
                let mensaje = match db_err.constraint() {
                    Some("proveedores_tipo_persona_check") => "el tipo de persona no es válido",
                    Some("proveedores_tipo_cuenta_check") => "el tipo de cuenta no es válido",
                    _ => "el tipo de proveedor no es válido",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

pub async fn add_proveedor<'e, E>(
    executor: E,
    datos: ProveedorNuevo,
) -> Result<Proveedor, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // id, codigo, activo, version, creado_en y actualizado_en se omiten a
    // propósito: los pone la tabla por DEFAULT (el consecutivo sale de la
    // secuencia, no de lo que mande el cliente) y vuelven en el RETURNING.
    let nuevo_proveedor = sqlx::query_as!(
        Proveedor,
        r#"
        INSERT INTO proveedores (tipo_proveedor, tipo_persona, tipo_documento, documento,
                                 digito_verificacion, nombre, razon_social,
                                 telefono, telefono_alterno, email, direccion,
                                 departamento, municipio, contacto_nombre, contacto_telefono,
                                 banco, tipo_cuenta, numero_cuenta, titular_cuenta,
                                 titular_documento, observaciones)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15,
                $16, $17, $18, $19, $20, $21)
        RETURNING id, codigo, tipo_proveedor, tipo_persona, tipo_documento, documento,
                  digito_verificacion, nombre, razon_social,
                  telefono, telefono_alterno, email, direccion,
                  departamento, municipio, contacto_nombre, contacto_telefono,
                  banco, tipo_cuenta, numero_cuenta, titular_cuenta,
                  titular_documento, observaciones,
                  activo, version, creado_en, actualizado_en
        "#,
        datos.tipo_proveedor,
        datos.tipo_persona,
        datos.tipo_documento,
        datos.documento,
        datos.digito_verificacion,
        datos.nombre,
        datos.razon_social,
        datos.telefono,
        datos.telefono_alterno,
        datos.email,
        datos.direccion,
        datos.departamento,
        datos.municipio,
        datos.contacto_nombre,
        datos.contacto_telefono,
        datos.banco,
        datos.tipo_cuenta,
        datos.numero_cuenta,
        datos.titular_cuenta,
        datos.titular_documento,
        datos.observaciones,
    )
    .fetch_one(executor)
    .await
    .map_err(map_conflicto_proveedor)?;

    Ok(nuevo_proveedor)
}

/// Listado filtrado. El SQL queda estático y cada filtro se apaga con un
/// `$n IS NULL`: así hay un solo plan que Postgres puede cachear y no se arma
/// la consulta concatenando texto.
pub async fn get_proveedores(
    pool: &PgPool,
    filtros: ProveedoresFiltros,
) -> Result<Vec<Proveedor>, ServiceError> {
    let proveedores = sqlx::query_as!(
        Proveedor,
        r#"
        SELECT id, codigo, tipo_proveedor, tipo_persona, tipo_documento, documento,
               digito_verificacion, nombre, razon_social,
               telefono, telefono_alterno, email, direccion,
               departamento, municipio, contacto_nombre, contacto_telefono,
               banco, tipo_cuenta, numero_cuenta, titular_cuenta,
               titular_documento, observaciones,
               activo, version, creado_en, actualizado_en
        FROM proveedores
        WHERE activo = $1
          AND ($2::text IS NULL OR tipo_proveedor = $2)
          AND ($3::text IS NULL OR tipo_persona = $3)
          -- Igualdad, no un LIKE: el departamento se filtra por el valor
          -- completo. Va por `lower()` en los dos lados porque la columna se
          -- guarda como la escribieron y "Antioquia" tiene que encontrar
          -- también a "ANTIOQUIA".
          AND ($4::text IS NULL OR lower(departamento) = lower($4))
          -- La búsqueda cubre lo que un humano tiene a mano del proveedor. El
          -- documento va por prefijo (se teclea de izquierda a derecha y así
          -- puede usar el índice); el resto por contenido. Los que no tienen
          -- razón social no se pierden: los otros OR se siguen evaluando.
          AND ($5::text IS NULL
               OR nombre       ILIKE '%' || $5 || '%'
               OR razon_social ILIKE '%' || $5 || '%'
               OR codigo       ILIKE '%' || $5 || '%'
               OR documento    ILIKE $5 || '%')
        ORDER BY creado_en DESC, id DESC
        LIMIT $6
        "#,
        filtros.activo,
        filtros.tipo_proveedor,
        filtros.tipo_persona,
        filtros.departamento,
        filtros.busqueda,
        filtros.limite,
    )
    .fetch_all(pool)
    .await?;

    Ok(proveedores)
}
