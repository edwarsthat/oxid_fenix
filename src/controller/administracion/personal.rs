
use crate::{
    models::{administracion::personal::PersonalReadPayload, validations::Validar},
    routes::protocol::{Ctx, WsResponse},
    services::administracion::personal::get_personal,
};

pub async fn personal_read(ctx: Ctx) -> WsResponse {
    let payload: PersonalReadPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let filtros = match payload.validar() {
        Ok(filtros) => filtros,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    match get_personal(&ctx.state.pool, filtros).await {
        Ok(personal) => WsResponse::ok(ctx.id, serde_json::json!({ "data": personal})),
        Err(err) => WsResponse::from_service_error(ctx.id, "personal_read", err),
    }
}

pub async fn personal_add(ctx: Ctx) -> WsResponse {
    todo!()
}