use crate::{
    models::{
        administracion::{sesion::SesionInfo, usuario::UsuarioIdPayload},
        validations::Validar,
    },
    routes::protocol::{Ctx, WsResponse},
};

pub async fn sessiones_read(ctx: Ctx) -> WsResponse {
    let sesiones: Vec<SesionInfo> = ctx
        .state
        .sessions
        .listar()
        .iter()
        .map(SesionInfo::from)
        .collect();

    WsResponse::ok(ctx.id, serde_json::json!({ "data": sesiones }))
}

pub async fn sessiones_delete(ctx: Ctx) -> WsResponse {
    let payload: UsuarioIdPayload =
        match serde_json::from_value(serde_json::Value::Object(ctx.data.clone())) {
            Ok(p) => p,
            Err(err) => return WsResponse::error(ctx.id, 400, &format!("Payload invalido: {err}")),
        };

    let usuario_id = match payload.validar() {
        Ok(id) => id,
        Err(err) => return WsResponse::error(ctx.id, 400, &format!("Datos invalidos: {err}")),
    };

    ctx.state.sessions.eliminar_por_usuario(usuario_id);

    ctx.emit(
        "sesiones",
        "delete",
        serde_json::json!({ "usuario_id": usuario_id }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": usuario_id }))
}
