use crate::app::app::AppState;
use crate::routes::protocol::{Ctx, WsRequest, WsResponse};
use serde_json;

pub async fn dispatch(raw: &str, state: &AppState) -> WsResponse {
    let req: WsRequest = match serde_json::from_str(raw) {
        Ok(req) => req,
        Err(_) => return WsResponse::error(String::new(), 400, "JSON inválido"),
    };

    // Primer nivel: el área. "sistema::auth::usuario::listar" -> ("sistema", "auth::usuario::listar")
    let (area, resto) = match req.action.split_once("::") {
        Some(par) => par,
        None => return WsResponse::error(req.id, 400, "action inválido"),
    };

    let ctx = Ctx {
        state: state.clone(),
        id: req.id,
        payload: req.payload,
    };

    match area {
        "sistema" => crate::routes::sistema::route(resto, ctx).await,
        _ => WsResponse::error(ctx.id, 404, "área desconocida"),
    }
}
