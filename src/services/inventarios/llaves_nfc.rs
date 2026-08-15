use crate::{
    models::inventarios::llaves_nfc::{LlaveNfc, LlaveNfcNueva},
    services::error::ServiceError,
};

/// Traduce los errores de integridad de la tabla `llaves_nfc` a errores de
/// negocio; si no, registrar dos veces la misma tarjeta saldría como 500.
fn map_conflicto_llave_nfc(err: sqlx::Error) -> ServiceError {
    if let sqlx::Error::Database(db_err) = &err {
        match db_err.code().as_deref() {
            // Hay dos UNIQUE en la tabla y el mensaje tiene que decir cuál
            // falló: el del uid lo provoca el usuario y lo puede corregir; el
            // del codigo solo pasa si la secuencia quedó desincronizada.
            Some("23505") => {
                let mensaje = match db_err.constraint() {
                    Some("llaves_nfc_codigo_key") => "ya existe una llave con ese código",
                    _ => "ya existe una llave registrada con ese uid",
                };
                return ServiceError::Conflict(mensaje.into());
            }
            // llaves_nfc_estado_check
            Some("23514") => {
                return ServiceError::Conflict("el estado de la llave no es válido".into());
            }
            _ => {}
        }
    }
    ServiceError::from(err)
}

pub async fn add_llave_nfc<'e, E>(
    executor: E,
    datos: LlaveNfcNueva,
) -> Result<LlaveNfc, ServiceError>
where
    E: sqlx::Executor<'e, Database = sqlx::Postgres>,
{
    // `codigo` y `estado` no se mandan a propósito: los pone la tabla (la
    // secuencia y el DEFAULT 'inventario'), así el consecutivo no depende de
    // lo que mande el cliente y ninguna llave nace ya dada de baja.
    let nueva_llave_nfc = sqlx::query_as!(
        LlaveNfc,
        r#"
        INSERT INTO llaves_nfc (uid, descripcion)
        VALUES ($1, $2)
        RETURNING id, uid, codigo, estado, descripcion, version,
                  creado_en, actualizado_en
        "#,
        datos.uid,
        datos.descripcion
    )
    .fetch_one(executor)
    .await
    .map_err(map_conflicto_llave_nfc)?;

    Ok(nueva_llave_nfc)
}
