use crate::routes::protocol::{WsRequest, WsResponse};

pub fn parsear_request(raw: &str) -> Result<WsRequest, WsResponse> {
    let req: WsRequest = match serde_json::from_str(raw) {
        Ok(req) => req,
        Err(_) => return Err(WsResponse::error(String::new(), 400, "JSON inválido")),
    };
    Ok(req)
}

pub fn partir_segmento<'a>(id: &str, action: &'a str) -> Result<(&'a str, &'a str), WsResponse> {
    let (area, resto) = match action.split_once(":") {
        Some(par) => par,
        None => return Err(WsResponse::error(id, 400, "action inválido")),
    };
    Ok((area, resto))
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn parsear_request_json_valido() {
        let raw = r#"{"id":"abc","token":"tok-1","payload":{"action":"sistema:auth:login","correo":"a@b.com"}}"#;

        let req = parsear_request(raw).expect("un JSON válido no debería fallar");

        assert_eq!(req.id, "abc");
        assert_eq!(req.token, "tok-1");
        assert_eq!(req.payload.action, "sistema:auth:login");
        assert_eq!(req.payload.data["correo"], "a@b.com");
    }

    #[test]
    fn parsear_request_payload_solo_con_action_deja_data_vacia() {
        // el caso real del cliente: payload trae action y nada más
        let raw = r#"{"id":"1","payload":{"action":"sistema:cargos:read"},"token":"tok-2"}"#;

        let req = parsear_request(raw).expect("un payload sin datos extra es válido");

        assert_eq!(req.payload.action, "sistema:cargos:read");
        assert!(req.payload.data.is_empty());
    }

    #[test]
    fn parsear_request_sin_payload_devuelve_400() {
        // payload es obligatorio: sin él no hay action, así que no hay request que enrutar
        let raw = r#"{"id":"abc","token":"tok-1"}"#;

        let err = parsear_request(raw).expect_err("sin payload debería devolver error");

        assert_eq!(err.status, 400);
        assert_eq!(err.message, "JSON inválido");
    }

    #[test]
    fn parsear_request_json_invalido() {
        let err = parsear_request("no soy json")
            .expect_err("un JSON malformado debería devolver error");

        assert_eq!(err.status, 400);
        assert_eq!(err.message, "JSON inválido");
        assert_eq!(err.id, "");
    }

    #[test]
    fn partir_segmento_con_separador() {
        let (area, resto) =
            partir_segmento("id-1", "sistema:auth:usuario:listar").expect("tiene ':'");

        assert_eq!(area, "sistema");
        assert_eq!(resto, "auth:usuario:listar");
    }

    #[test]
    fn partir_segmento_corta_en_la_primera_ocurrencia() {
        // split_once parte solo en el primer ":", el resto queda intacto
        let (area, resto) = partir_segmento("id-1", "sistema:").expect("tiene ':'");

        assert_eq!(area, "sistema");
        assert_eq!(resto, "");
    }

    #[test]
    fn partir_segmento_sin_separador() {
        let err = partir_segmento("id-1", "sistema").expect_err("sin ':' debería devolver error");

        assert_eq!(err.status, 400);
        assert_eq!(err.message, "action inválido");
        assert_eq!(err.id, "id-1");
    }
}

