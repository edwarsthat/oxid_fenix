use crate::controller::sistema::auth as controller;
use crate::routes::protocol::{Ctx, WsResponse};

/// Router del dominio `auth` (dentro del área `sistema`).
/// resto: "usuario:listar"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    println!("[sistema::auth] ruta recibida: '{resto}' (id={})", ctx.id);

    match resto {
        "usuario:listar" => controller::list_usuarios(ctx).await,
        _ => {
            tracing::warn!("[sistema::auth] !! acción desconocida: '{resto}'");
            WsResponse::error(ctx.id, 404, "Acción desconocida")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        app::app::AppState,
        sessions::memory::SessionStore
    };
    use sqlx::PgPool;
    use std::collections::HashSet;
    use std::sync::Arc;

    fn ctx_de_prueba(id: &str) -> Ctx {
        let pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        Ctx {
            state: AppState { pool, sessions: SessionStore::new() },
            id: id.to_string(),
            data: serde_json::Map::new(),
            token: "token-de-prueba".to_string(),
            permisos: Arc::new(HashSet::new()),
        }
    }

    #[tokio::test]
    async fn route_usuario_listar_devuelve_ok() {
        let resp = route("usuario:listar", ctx_de_prueba("id-1")).await;
        assert_eq!(resp.id, "id-1");
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn route_accion_desconocida_devuelve_404() {
        let resp = route("otra:cosa", ctx_de_prueba("id-2")).await;

        assert_eq!(resp.id, "id-2");
        assert_eq!(resp.status, 404);
        assert_eq!(resp.message, "Acción desconocida");
    }
}