use std::collections::HashSet;
use std::sync::Arc;

use uuid::Uuid;

use crate::app::app::AppState;
use crate::routes::actions::{parsear_request, partir_segmento};
use crate::routes::protocol::{Ctx, WsRequest, WsResponse};

/// Acciones `(area, resto)` que siguen disponibles mientras la sesión arrastre
/// `debe_cambiar_password`. El cambio en sí va por HTTP (`/cambiar-password`),
/// así que no pasa por aquí y no hace falta listarlo.
const ACCIONES_PASSWORD_PENDIENTE: &[(&str, &str)] = &[("sistema", "auth:logout")];

pub async fn dispatch(raw: &str, state: &AppState) -> WsResponse {
    let req: WsRequest = match parsear_request(raw) {
        Ok(req) => req,
        Err(err) => return err,
    };

    //se obtiene la sesion
    let sesion = match Uuid::parse_str(&req.token) {
        Ok(token) => state.sessions.validar(&token),
        Err(_) => None, // token mal formado → sin sesión
    };

    //obtiene el area o modulo
    let (area, resto) = match partir_segmento(&req.id, &req.payload.action) {
        Ok((area, resto)) => (area, resto),
        Err(err) => return err,
    };

    //revisa permisos y autentificacion
    let (permisos, actor_id) = match sesion {
        // Con la contraseña temporal pendiente la sesión no sirve para nada más:
        // se corta aquí, en el único punto por el que pasan todas las áreas.
        Some(s)
            if s.debe_cambiar_password && !ACCIONES_PASSWORD_PENDIENTE.contains(&(area, resto)) =>
        {
            return WsResponse::error(req.id, 403, "debe cambiar la contraseña");
        }
        Some(s) => (s.permisos, s.usuario_id),
        None if area == "sistema" => (Arc::new(HashSet::new()), Uuid::nil()),
        None => return WsResponse::error(req.id, 401, "no autenticado"),
    };

    let ctx = Ctx {
        state: state.clone(),
        id: req.id,
        data: req.payload.data,
        token: req.token,
        permisos: permisos,
        user_id: actor_id,
    };

    match area {
        "sistema" => crate::routes::sistema::route(resto, ctx).await,
        "administracion" => crate::routes::administracion::route(resto, ctx).await,
        _ => WsResponse::error(ctx.id, 404, "área desconocida"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::memory::SessionStore;
    use chrono::Duration;
    use sqlx::PgPool;
    use std::collections::HashSet;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    fn permisos(items: &[&str]) -> Arc<HashSet<String>> {
        Arc::new(items.iter().map(|p| p.to_string()).collect())
    }

    fn state_de_prueba() -> AppState {
        let pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        AppState {
            pool,
            sessions: SessionStore::new(),
            eventos: broadcast::Sender::new(100),
        }
    }

    /// State con una sesión viva y los permisos dados; devuelve el token con el
    /// que autenticarse.
    fn state_con_sesion(permitidos: &[&str]) -> (AppState, Uuid) {
        state_con_sesion_flag(permitidos, false)
    }

    fn state_con_sesion_flag(permitidos: &[&str], debe_cambiar_password: bool) -> (AppState, Uuid) {
        let state = state_de_prueba();
        let token = state.sessions.crear(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Duration::hours(1),
            permisos(permitidos),
            debe_cambiar_password,
        );

        (state, token)
    }

    fn raw(id: &str, token: &str, action: &str) -> String {
        format!(r#"{{"id":"{id}","token":"{token}","payload":{{"action":"{action}"}}}}"#)
    }

    #[tokio::test]
    async fn dispatch_json_invalido_devuelve_400() {
        let state = state_de_prueba();

        let resp = dispatch("no soy json", &state).await;

        assert_eq!(resp.status, 400);
        assert_eq!(resp.message, "JSON inválido");
    }

    #[tokio::test]
    async fn dispatch_action_sin_separador_devuelve_400() {
        let (state, token) = state_con_sesion(&[]);
        let raw = raw("id-1", &token.to_string(), "sistema");

        let resp = dispatch(&raw, &state).await;

        assert_eq!(resp.id, "id-1");
        assert_eq!(resp.status, 400);
        assert_eq!(resp.message, "action inválido");
    }

    #[tokio::test]
    async fn dispatch_area_desconocida_devuelve_404() {
        let (state, token) = state_con_sesion(&["algo"]);
        let raw = raw("id-2", &token.to_string(), "otraarea:algo");

        let resp = dispatch(&raw, &state).await;

        assert_eq!(resp.id, "id-2");
        assert_eq!(resp.status, 404);
        assert_eq!(resp.message, "área desconocida");
    }

    #[tokio::test]
    async fn dispatch_sistema_auth_usuario_listar_devuelve_ok() {
        // sistema es el área pública: se enruta aunque no haya sesión
        let state = state_de_prueba();
        let raw = raw("id-3", "sin-sesion", "sistema:auth:usuario:listar");

        let resp = dispatch(&raw, &state).await;

        assert_eq!(resp.id, "id-3");
        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn dispatch_con_password_pendiente_bloquea_area_privada() {
        // tiene el permiso, pero la contraseña temporal sigue sin cambiarse
        let (state, token) = state_con_sesion_flag(&["cargos:read"], true);
        let raw = raw("id-5", &token.to_string(), "administracion:cargos:read");

        let resp = dispatch(&raw, &state).await;

        assert_eq!(resp.id, "id-5");
        assert_eq!(resp.status, 403);
        assert_eq!(resp.message, "debe cambiar la contraseña");
    }

    #[tokio::test]
    async fn dispatch_con_password_pendiente_bloquea_area_publica() {
        // sistema no revisa permisos, así que el corte tiene que venir del flag
        let (state, token) = state_con_sesion_flag(&[], true);
        let raw = raw("id-6", &token.to_string(), "sistema:auth:usuario:listar");

        let resp = dispatch(&raw, &state).await;

        assert_eq!(resp.status, 403);
        assert_eq!(resp.message, "debe cambiar la contraseña");
    }

    #[tokio::test]
    async fn dispatch_con_password_pendiente_permite_logout() {
        let (state, token) = state_con_sesion_flag(&[], true);
        let raw = raw("id-7", &token.to_string(), "sistema:auth:logout");

        let resp = dispatch(&raw, &state).await;

        assert_eq!(resp.status, 200);
        assert!(state.sessions.validar(&token).is_none());
    }

    #[tokio::test]
    async fn dispatch_sin_password_pendiente_no_bloquea() {
        let (state, token) = state_con_sesion_flag(&[], false);
        let raw = raw("id-8", &token.to_string(), "sistema:auth:usuario:listar");

        let resp = dispatch(&raw, &state).await;

        assert_eq!(resp.status, 200);
    }

    #[tokio::test]
    async fn dispatch_area_privada_sin_sesion_devuelve_401() {
        // token con forma de Uuid, pero que no existe en el store
        let state = state_de_prueba();
        let raw = raw(
            "id-4",
            &Uuid::new_v4().to_string(),
            "administracion:cargos:listar",
        );

        let resp = dispatch(&raw, &state).await;

        assert_eq!(resp.id, "id-4");
        assert_eq!(resp.status, 401);
        assert_eq!(resp.message, "no autenticado");
    }
}
