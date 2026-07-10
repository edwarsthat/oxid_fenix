use crate::app::app::AppState;
use crate::routes::actions::{parsear_request, partir_segmento};
use crate::routes::protocol::{Ctx, WsRequest, WsResponse};

pub async fn dispatch(raw: &str, state: &AppState) -> WsResponse {
    let req: WsRequest = match parsear_request(raw) {
        Ok(req) => req,
        Err(err) => return err
    };

    let (area, resto) = match partir_segmento(&req.id, &req.action) {
        Ok((area, resto)) => (area, resto),
        Err(err) => return err
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
