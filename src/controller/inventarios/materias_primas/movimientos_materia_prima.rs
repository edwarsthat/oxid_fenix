use std::sync::Arc;

use axum::{Extension, Json, extract::State, http::StatusCode};

use crate::{
    app::{app::AppState, error::ApiError}, models::{
        dispositivos::concentradores::Concentrador,
        inventarios::materia_prima::movimientos_materia_prima::{
            ConsumoBasculaNuevo, PesajeBasculaPayload, PesajesReadPayload,
        },
        validations::Validar,
    }, routes::protocol::{Ctx, Evento, WsEvent, WsResponse}, services::{
        dispositivos::basculas::bascula_por_device_id, error::ServiceError, inventarios::{
            llaves_nfc::get_empleado_con_llave_en, materias_primas::movimientos_materia_prima::{add_consumo_bascula, existe_clave_idempotencia, get_pesajes_bascula, get_totales_pesajes_bascula},
        }, logs::audit_logs::create_audit_log, proceso::programacion_proceso::get_programacion_en,
    },
};

/// En este endpoint un error de negocio no puede salir como 500: la Pi lo
/// reintentaría para siempre y frenaría todos los pesajes de atrás. Solo lo
/// pasajero (la base) se deja como 500, para que sí lo reintente.
fn rechazo_o_interno(err: ServiceError) -> ApiError {
    match err {
        ServiceError::NotFound(msg)
        | ServiceError::Conflict(msg)
        | ServiceError::BadRequest(msg) => ApiError::PesajeRechazado(msg),
        otro => ApiError::Service(otro),
    }
}

/// POST /api/pesajes: un pesaje de báscula reenviado por el concentrador.
///
/// El concentrador ya viene autenticado por el middleware
/// `autenticar_concentrador`, que lo deja en las extensions.
///
/// Respuestas, según lo que la Pi hace con cada una:
/// - 201: guardado. 409: ya estaba (reintento); la Pi lo toma como éxito.
/// - 400 / 422: el pesaje no sirve o no se pudo asignar; la Pi lo aparta.
/// - 500: falló la base; la Pi lo reintenta.
pub async fn ingreso_movimiento_materia_prima(
    State(state): State<AppState>,
    Extension(concentrador): Extension<Arc<Concentrador>>,
    Json(payload): Json<PesajeBasculaPayload>,
) -> Result<StatusCode, ApiError> {
    let pesaje = payload.validar()?;

    let mut tx = state
        .pool
        .begin()
        .await
        .map_err(|err| ApiError::Service(err.into()))?;

    if existe_clave_idempotencia(&mut *tx, pesaje.clave_idempotencia)
        .await
        .map_err(rechazo_o_interno)?
    {
        return Ok(StatusCode::CONFLICT);
    }

    // Todo se resuelve con `medido_en` y no con la hora de llegada: el pesaje
    // puede llegar horas tarde por la cola de la Pi, y para entonces la línea
    // puede estar en otro lote y la llave en otras manos.
    let bascula = bascula_por_device_id(&mut *tx, &pesaje.device_id, concentrador.id)
        .await
        .map_err(rechazo_o_interno)?
        .ok_or_else(|| {
            ApiError::PesajeRechazado(format!(
                "la báscula {} no está registrada en el concentrador {}",
                pesaje.device_id, concentrador.codigo
            ))
        })?;

    let programacion = get_programacion_en(&mut *tx, bascula.linea, pesaje.medido_en)
        .await
        .map_err(rechazo_o_interno)?
        .ok_or_else(|| {
            ApiError::PesajeRechazado(format!(
                "la línea {} no tenía lote programado a las {}",
                bascula.linea, pesaje.medido_en
            ))
        })?;

    let operario_id = get_empleado_con_llave_en(&mut *tx, &pesaje.uid_tarjeta, pesaje.medido_en)
        .await
        .map_err(rechazo_o_interno)?
        .ok_or_else(|| {
            ApiError::PesajeRechazado(format!(
                "la tarjeta {} no estaba asignada a nadie a las {}",
                pesaje.uid_tarjeta, pesaje.medido_en
            ))
        })?;

    let datos = ConsumoBasculaNuevo {
        clave_idempotencia: pesaje.clave_idempotencia,
        lote_id: programacion.lote_id,
        programacion_id: programacion.id,
        operario_id,
        peso: pesaje.peso_kg,
        medido_en: pesaje.medido_en,
        recibido_concentrador_en: pesaje.recibido_concentrador_en,
        concentrador_id: concentrador.id,
        bascula_id: bascula.id,
        sesion_arranque: pesaje.sesion_arranque,
        transaccion: pesaje.transaccion,
        uid_tarjeta: pesaje.uid_tarjeta,
    };

    // `registrado_por` es el usuario de sistema: ninguna persona metió este
    // pesaje. Quién pesó lo dice `operario_id`.
    let alta = add_consumo_bascula(&mut tx, datos, state.usuario_basculas_id)
        .await
        .map_err(rechazo_o_interno)?;

    // El reintento devuelve el movimiento que ya estaba, y ahí se corta: volver
    // a auditar y a emitir contaría dos veces el mismo pesaje en la pantalla.
    if alta.creado {
        create_audit_log(
            &mut *tx,
            "movimientos_materia_prima",
            alta.movimiento.id,
            "add",
            state.usuario_basculas_id,
            Some("inventarios"),
            Some(serde_json::json!({
                "tipo": alta.movimiento.tipo,
                "lote_id": alta.movimiento.lote_id,
                "programacion_id": alta.movimiento.programacion_id,
                "operario_id": alta.movimiento.operario_id,
                "peso": alta.movimiento.peso,
                "medido_en": alta.movimiento.medido_en,
                "bascula_id": alta.movimiento.bascula_id,
                "concentrador_id": alta.movimiento.concentrador_id,
            })),
        )
        .await
        .map_err(ApiError::Service)?;
    }

    tx.commit()
        .await
        .map_err(|err| ApiError::Service(err.into()))?;

    if !alta.creado {
        return Ok(StatusCode::CONFLICT);
    }

    // Sin `Ctx` no hay `ctx.emit`: se arma el mismo evento a mano. El permiso
    // sigue la convención de `emit` (`{event}:read`), así lo reciben las
    // pantallas que ya escuchan el inventario de materia prima.
    let evento = WsEvent {
        event: "inventario_materia_prima".into(),
        action: "consumo".into(),
        data: serde_json::json!({ "data": alta.movimiento }),
    };
    if let Ok(json) = serde_json::to_string(&evento) {
        let _ = state.eventos.send(Arc::new(Evento {
            permiso: "inventario_materia_prima:read".into(),
            json,
        }));
    }

    Ok(StatusCode::CREATED)
}

/// Listado de pesajes de báscula para la pantalla del destajo: una página del
/// listado y los totales por operario del mismo filtro.
///
/// Los totales solo se calculan en la primera página (sin `cursor`): no
/// dependen de la página, y repetir el GROUP BY de hasta 31 días cada vez que
/// el usuario baja el scroll es trabajo tirado. En las páginas siguientes van
/// en `null` y el front se queda con los que ya tiene; para refrescarlos pide
/// de nuevo la primera página.
pub async fn pesajes_bascula_read(ctx: Ctx) -> WsResponse {
    let payload: PesajesReadPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let filtros = match payload.validar() {
        Ok(filtros) => filtros,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    let totales = if filtros.cursor.is_none() {
        match get_totales_pesajes_bascula(&ctx.state.pool, &filtros).await {
            Ok(totales) => Some(totales),
            Err(err) => {
                return WsResponse::from_service_error(ctx.id, "pesajes_bascula_read", err);
            }
        }
    } else {
        None
    };

    match get_pesajes_bascula(&ctx.state.pool, filtros).await {
        Ok(pagina) => WsResponse::ok(
            ctx.id,
            serde_json::json!({
                "data": {
                    "pesajes": pagina.pesajes,
                    "siguiente_cursor": pagina.siguiente_cursor,
                    "totales": totales,
                }
            }),
        ),
        Err(err) => WsResponse::from_service_error(ctx.id, "pesajes_bascula_read", err),
    }
}
