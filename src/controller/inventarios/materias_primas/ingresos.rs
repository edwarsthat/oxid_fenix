use crate::{
    models::{inventarios::materia_prima::ingresos::IngresoAddPayload, validations::Validar},
    routes::protocol::{Ctx, WsResponse},
    services::{
        inventarios::materias_primas::ingresos::add_ingreso_lote_materia_prima,
        logs::audit_logs::create_audit_log,
    },
};

pub async fn ingreso_lote_materia_prima_add(ctx: Ctx) -> WsResponse {
    let payload: IngresoAddPayload =
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
            return WsResponse::internal_error(ctx.id, "lotes_materias_primas_add", err);
        }
    };

    // `registrado_por` sale de la sesión, no del payload: quién registró el
    // lote es lo que se mira cuando el peso no cuadra contra el tiquete.
    let alta = match add_ingreso_lote_materia_prima(&mut tx, datos, ctx.user_id).await {
        Ok(alta) => alta,
        Err(err) => {
            return WsResponse::from_service_error(ctx.id, "lotes_materias_primas_add", err);
        }
    };

    // El reintento de un alta que ya funcionó devuelve el mismo lote, y ahí se
    // corta: volver a auditar y a emitir pintaría el mismo camión dos veces en
    // la pantalla de patio y dejaría dos "add" en el log para un solo ingreso.
    // La transacción se cierra igual, con el commit de abajo, porque el SELECT
    // del reintento se hizo dentro de ella.
    if alta.creado
        && let Err(err) = create_audit_log(
            &mut *tx,
            "ingresos_materia_prima",
            alta.ingreso.id,
            "add",
            ctx.user_id,
            Some("inventarios"),
            Some(serde_json::json!({
                "codigo": alta.ingreso.codigo,
                "predio_id": alta.ingreso.predio_id,
                "materia_prima_id": alta.ingreso.materia_prima_id,
                "placa": alta.ingreso.placa,
                "numero_remision": alta.ingreso.numero_remision,
                "numero_tiquete_bascula": alta.ingreso.numero_tiquete_bascula,
                "fecha_ingreso": alta.ingreso.fecha_ingreso,
                "llegada_en": alta.ingreso.llegada_en,
                "peso_ingreso": alta.ingreso.peso_ingreso,
                "peso_devuelto": alta.ingreso.peso_devuelto,
            })),
        )
        .await
    {
        return WsResponse::from_service_error(ctx.id, "lotes_materias_primas_add", err);
    }

    if let Err(err) = tx.commit().await {
        return WsResponse::internal_error(ctx.id, "lotes_materias_primas_add", err);
    }

    // El evento tiene que llamarse igual que el permiso: `emit` arma el filtro
    // como `{event}:read` y el permiso sembrado es `lotes_materias_primas:read`.
    if alta.creado {
        ctx.emit(
            "lotes_materias_primas",
            "add",
            serde_json::json!({ "data": alta.ingreso }),
        );
    }

    // El reintento responde 200 con el mismo lote, no un 409: para el de
    // báscula la petición que se le perdió terminó bien, y no tiene nada que
    // corregir. `creado` va en la respuesta por si el cliente quiere
    // distinguirlos (mostrar "ya estaba registrado" en vez de "registrado").
    WsResponse::ok(
        ctx.id,
        serde_json::json!({ "data": alta.ingreso, "creado": alta.creado }),
    )
}
