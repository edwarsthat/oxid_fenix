use uuid::Uuid;

use crate::{
    models::administracion::sesion::SesionInfo,
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
    let usuario_id = match ctx.data.get("usuario_id").and_then(|v| v.as_str()) {
        Some(usuario_id) => usuario_id,
        None => return WsResponse::error(ctx.id, 400, &format!("Falta Usuario Id")),
    };

    let usuario_id = match Uuid::parse_str(&usuario_id) {
        Ok(id) => id,
        Err(_) => return WsResponse::error(ctx.id, 400, &format!("Usuario_id no valido")),
    };

    ctx.state.sessions.eliminar_por_usuario(usuario_id);

    ctx.emit(
        "sesiones",
        "delete",
        serde_json::json!({ "usuario_id": usuario_id }),
    );

    WsResponse::ok(ctx.id, serde_json::json!({ "data": usuario_id }))
}
