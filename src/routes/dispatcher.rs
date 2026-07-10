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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::memory::SessionStore;
    use sqlx::PgPool;

    fn state_de_prueba() -> AppState {
        let pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        AppState {
            pool,
            sessions: SessionStore::new()
        }
    }

    #[tokio::test]
    async fn dispatch_json_invalido_devuelve_400(){
        let state = state_de_prueba();

        let resp = dispatch("no soy json", &state).await;

        assert_eq!(resp.status, 400);
        assert_eq!(resp.message, "JSON inválido");
    }

#[tokio::test]
    async fn dispatch_area_desconocida_devuelve_404() {
        let state = state_de_prueba();
        let raw = r#"{"id":"id-1","action":"otraarea::algo"}"#;

        let resp = dispatch(raw, &state).await;

        assert_eq!(resp.id, "id-1");
        assert_eq!(resp.status, 404);
        assert_eq!(resp.message, "área desconocida");
    }

    #[tokio::test]
    async fn dispatch_sistema_auth_usuario_listar_devuelve_ok() {
        let state = state_de_prueba();
        let raw = r#"{"id":"id-2","action":"sistema::auth::usuario::listar"}"#;

        let resp = dispatch(raw, &state).await;

        assert_eq!(resp.id, "id-2");
        assert_eq!(resp.status, 200);
    }
}