//! Autenticación de los concentradores (Raspberry Pi) por API key.
//!
//! La Pi manda `Authorization: Bearer <llave>` en cada petición. Aquí se
//! calcula el SHA-256 de la llave y se busca el concentrador activo con ese
//! hash. La llave en claro no sale de este módulo: el servicio solo ve el hash.
//!
//! SHA-256 y no argon2 a propósito: la llave es aleatoria y larga, así que no
//! hay diccionario contra el que probar, y un hash determinista es lo que
//! permite buscarla con `WHERE llave_hash = $1`.

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::{HeaderMap, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use sha2::{Digest, Sha256};

use crate::app::{app::AppState, error::ApiError};
use crate::services::dispositivos::concentradores::concentrador_por_llave_hash;

/// SHA-256 de la llave en hex minúscula: 64 caracteres, el mismo formato que
/// la columna `concentradores.llave_hash`.
pub fn hash_llave(llave: &str) -> String {
    format!("{:x}", Sha256::digest(llave.as_bytes()))
}

fn extraer_llave(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|llave| !llave.is_empty())
        .ok_or(ApiError::TokenAusente)
}

/// Middleware para las rutas de la Pi. Si la llave es válida deja el
/// `Arc<Concentrador>` en las extensions de la petición, de donde lo saca el
/// handler con `Extension`.
pub async fn autenticar_concentrador(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let llave = match extraer_llave(req.headers()) {
        Ok(llave) => llave,
        Err(err) => return err.into_response(),
    };
    let llave_hash = hash_llave(llave);

    let concentrador = match concentrador_por_llave_hash(&state.pool, &llave_hash).await {
        Ok(Some(concentrador)) => concentrador,
        // llave desconocida o concentrador dado de baja: misma respuesta, para
        // no contarle a nadie cuál de las dos fue
        Ok(None) => return ApiError::TokenInvalido.into_response(),
        Err(err) => return ApiError::Service(err).into_response(),
    };

    req.extensions_mut().insert(Arc::new(concentrador));
    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn headers_con_auth(valor: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::AUTHORIZATION, HeaderValue::from_str(valor).unwrap());
        headers
    }

    // ── hash_llave ──────────────────────────────────
    #[test]
    fn hash_llave_coincide_con_sha256_conocido() {
        // vector de prueba estándar de SHA-256
        assert_eq!(
            hash_llave("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn hash_llave_tiene_el_formato_de_la_columna() {
        // CHAR(64) en hex minúscula
        let hash = hash_llave("cualquier-llave");
        assert_eq!(hash.len(), 64);
        assert!(
            hash.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        );
    }

    #[test]
    fn hash_llave_distingue_llaves_distintas() {
        assert_ne!(hash_llave("llave-1"), hash_llave("llave-2"));
    }

    // ── extraer_llave ───────────────────────────────
    #[test]
    fn extraer_llave_sin_header_es_ausente() {
        let err = extraer_llave(&HeaderMap::new()).unwrap_err();
        assert!(matches!(err, ApiError::TokenAusente));
    }

    #[test]
    fn extraer_llave_sin_prefijo_bearer_es_ausente() {
        let err = extraer_llave(&headers_con_auth("mi-llave")).unwrap_err();
        assert!(matches!(err, ApiError::TokenAusente));
    }

    #[test]
    fn extraer_llave_vacia_es_ausente() {
        let err = extraer_llave(&headers_con_auth("Bearer   ")).unwrap_err();
        assert!(matches!(err, ApiError::TokenAusente));
    }

    #[test]
    fn extraer_llave_con_bearer_devuelve_la_llave() {
        assert_eq!(
            extraer_llave(&headers_con_auth("Bearer mi-llave")).unwrap(),
            "mi-llave"
        );
    }
}
