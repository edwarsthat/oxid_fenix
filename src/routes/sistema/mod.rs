pub mod auth;

use crate::routes::{protocol::{Ctx, WsResponse}};

/// Router del área `sistema`. Reparte por dominio.
/// resto: "auth:usuario:listar" -> dominio="auth", resto="usuario:listar"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    let (dominio, resto) = match resto.split_once(":") {
        Some(par) => par,
        None => return WsResponse::error(ctx.id, 400, "action inválido"),
    };

    match dominio {
        "auth" => auth::route(resto, ctx).await,
        _ => WsResponse::error(ctx.id, 404, "dominio desconocido"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{app::app::AppState, sessions::memory::SessionStore};
    use sqlx::PgPool;
    use uuid::Uuid;
    use std::collections::HashSet;
    use std::sync::Arc;

    fn ctx_de_prueba(id: &str) -> Ctx {
        // connect_lazy no abre conexión real: sirve porque estas rutas no tocan el pool
        let pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        Ctx {
            state: AppState { pool, sessions: SessionStore::new() },
            id: id.to_string(),
            data: serde_json::Map::new(),
            token: "token-de-prueba".to_string(),
            permisos: Arc::new(HashSet::new()),
            user_id: Uuid::new_v4()
        }
    }

    #[tokio::test]
    async fn route_dominio_auth_delega_correctamente() {
        let resp = route("auth:usuario:listar", ctx_de_prueba("id-1")).await;

        assert_eq!(resp.id, "id-1");
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn route_dominio_desconocido_devuelve_404() {
        let resp = route("otro:algo", ctx_de_prueba("id-2")).await;

        assert_eq!(resp.id, "id-2");
        assert_eq!(resp.status, 404);
        assert_eq!(resp.message, "dominio desconocido");
    }

    #[tokio::test]
    async fn route_sin_separador_devuelve_400() {
        let resp = route("auth", ctx_de_prueba("id-3")).await;

        assert_eq!(resp.id, "id-3");
        assert_eq!(resp.status, 400);
        assert_eq!(resp.message, "action inválido");
    }
}
