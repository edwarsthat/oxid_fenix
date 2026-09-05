use crate::{
    models::{proceso::programacion_proceso::ProgramacionProcesoAddPayload, validations::Validar},
    routes::protocol::{Ctx, WsResponse},
    services::{
        logs::audit_logs::create_audit_log, proceso::programacion_proceso::add_programacion_proceso,
    },
};

pub async fn programacion_proceso_add(ctx: Ctx) -> WsResponse {
    let payload: ProgramacionProcesoAddPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let datos = match payload.validar() {
        Ok(datos) => datos,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    let mut tx = match ctx.state.pool.begin().await {
        Ok(tx) => tx,
        Err(err) => {
            return WsResponse::internal_error(ctx.id, "programaciones_proceso_add", err);
        }
    };

    // `programado_por` sale de la sesión, no del payload: quién montó el lote es
    // lo primero que se mira cuando las pesadas de un turno quedaron en el lote
    // equivocado.
    let alta = match add_programacion_proceso(&mut tx, datos, ctx.user_id).await {
        Ok(alta) => alta,
        Err(err) => {
            return WsResponse::from_service_error(ctx.id, "programaciones_proceso_add", err);
        }
    };

    // El reintento de una programación que ya estaba montada devuelve la misma
    // fila, y ahí se corta: volver a auditar y a emitir dejaría dos "add" en el
    // log para un solo montaje.
    //
    // El cierre de la programación anterior no lleva renglón propio en la
    // auditoría: la fila cerrada ya guarda `fin_en` y `cerrado_por`, así que
    // quién la cerró y cuándo se lee de ella misma.
    if alta.creado
        && let Err(err) = create_audit_log(
            &mut *tx,
            "programaciones_proceso",
            alta.programacion.id,
            "add",
            ctx.user_id,
            Some("proceso"),
            Some(serde_json::json!({
                "lote_id": alta.programacion.lote_id,
                "linea": alta.programacion.linea,
                "inicio_en": alta.programacion.inicio_en,
                "observaciones": alta.programacion.observaciones,
            })),
        )
        .await
    {
        return WsResponse::from_service_error(ctx.id, "programaciones_proceso_add", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "programaciones_proceso_add", err);
    }

    // El evento tiene que llamarse igual que el permiso: `emit` arma el filtro
    // como `{event}:read`, así que esto pide `programaciones_proceso:read`.
    if alta.creado {
        ctx.emit(
            "programaciones_proceso",
            "add",
            serde_json::json!({ "data": alta.programacion }),
        );
    }

    // El reintento responde 200 con la misma programación, no un 409: para el
    // coordinador la petición que se le perdió terminó bien y no tiene nada que
    // corregir. `creado` va en la respuesta por si el cliente quiere distinguir
    // ("ya estaba montado" en vez de "montado").
    WsResponse::ok(
        ctx.id,
        serde_json::json!({ "data": alta.programacion, "creado": alta.creado }),
    )
}
