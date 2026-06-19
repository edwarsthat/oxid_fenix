pub mod auth;

use crate::routes::protocol::{Ctx, WsResponse};

/// Router del área `sistema`. Reparte por dominio.
/// resto: "auth::usuario::listar" -> dominio="auth", resto="usuario::listar"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    let (dominio, resto) = match resto.split_once("::") {
        Some(par) => par,
        None => return WsResponse::error(ctx.id, 400, "action inválido"),
    };

    match dominio {
        "auth" => auth::route(resto, ctx).await,
        _ => WsResponse::error(ctx.id, 404, "dominio desconocido"),
    }
}
