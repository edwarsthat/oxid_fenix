use crate::{
    models::proveedores::predios::{PredioNuevo, Predios},
    services::error::ServiceError,
};

/// Traduce los errores de integridad de la tabla `predios` a errores de
/// negocio; si no, cargar dos veces la misma finca saldría como 500.
fn map_conflicto_predio(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            // Los dos UNIQUE de la tabla. El del nombre lo provoca el usuario y
            // lo puede corregir; el del codigo solo pasa si la secuencia quedó
            // desincronizada.
            Some("23505") => {
                let mensaje = match db_err.constraint() {
                    Some("predios_codigo_key") => "ya existe un predio con ese código",
                    _ => "el proveedor ya tiene un predio registrado con ese nombre",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            // La única FK de la tabla es la del proveedor: el id llega en el
            // payload, así que un proveedor borrado o mal escrito cae acá.
            Some("23503") => {
                return ServiceError::NotFound("el proveedor indicado no existe".into());
            }
            // Los tres CHECK de la coordenada. El payload ya los valida con las
            // mismas reglas, así que llegar acá significa que alguien insertó
            // sin pasar por `PredioAddPayload`: igual conviene un mensaje que
            // diga qué columna, y no un 23514 pelado.
            Some("23514") => {
                let mensaje = match db_err.constraint() {
                    Some("predios_latitud_check") => "la latitud está fuera de rango",
                    Some("predios_longitud_check") => "la longitud está fuera de rango",
                    _ => "la latitud y la longitud van juntas: falta una de las dos",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

pub async fn add_predios<'e, E>(executor: E, datos: PredioNuevo) -> Result<Predios, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // id, codigo, activo, version, creado_en y actualizado_en se omiten a
    // propósito: los pone la tabla por DEFAULT (el consecutivo sale de la
    // secuencia, no de lo que mande el cliente) y vuelven en el RETURNING.
    //
    // Las coordenadas van y vuelven casteadas: la columna es NUMERIC y el
    // driver no trae tipo decimal habilitado, así que el parámetro entra como
    // float8 y el RETURNING las devuelve igual.
    let nuevo_predio = sqlx::query_as!(
        Predios,
        r#"
        INSERT INTO predios (proveedor_id, nombre, departamento, municipio, vereda,
                             referencia_ubicacion, latitud, longitud,
                             responsable_nombre, responsable_documento, responsable_telefono,
                             observaciones)
        VALUES ($1, $2, $3, $4, $5, $6, $7::float8::numeric, $8::float8::numeric,
                $9, $10, $11, $12)
        RETURNING id, codigo, proveedor_id, nombre, departamento, municipio, vereda,
                  referencia_ubicacion,
                  latitud::float8  AS "latitud",
                  longitud::float8 AS "longitud",
                  responsable_nombre, responsable_documento, responsable_telefono,
                  observaciones, activo, version, creado_en, actualizado_en
        "#,
        datos.proveedor_id,
        datos.nombre,
        datos.departamento,
        datos.municipio,
        datos.vereda,
        datos.referencia_ubicacion,
        datos.latitud,
        datos.longitud,
        datos.responsable_nombre,
        datos.responsable_documento,
        datos.responsable_telefono,
        datos.observaciones,
    )
    .fetch_one(executor)
    .await
    .map_err(map_conflicto_predio)?;

    Ok(nuevo_predio)
}
