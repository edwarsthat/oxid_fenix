use crate::{
    routes::protocol::{Ctx, WsResponse},
    services::administracion::cargos::{create_cargo, get_cargos},
};

pub async fn cargos_read(ctx: Ctx) -> WsResponse {
    match get_cargos(&ctx.state.pool).await {
        Ok(cargos) => WsResponse::ok(ctx.id, serde_json::json!({ "data": cargos })),
        Err(err) => {
            eprintln!("[cargos_read] {err}");
            WsResponse::error(ctx.id, 500, "error interno")
        }
    }
}

pub async fn cargos_add(ctx: Ctx) -> WsResponse {
    let nombre = match ctx.data.get("nombre").and_then(|v| v.as_str()) {
        Some(nombre) => nombre,
        None => return WsResponse::error(ctx.id, 403, "Error nombre no valido"),
    };

    let descripcion = match ctx.data.get("descripcion").and_then(|v| v.as_str()) {
        Some(nombre) => nombre,
        None => return WsResponse::error(ctx.id, 403, "Error descripcion no valida"),
    };

    let permisos_arr = match ctx.data.get("permisos").and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return WsResponse::error(ctx.id, 403, "Error permisos no validos"),
    };

    let permisos: Vec<String> = match permisos_arr
        .iter()
        .map(|v| v.as_str().map(String::from))
        .collect::<Option<Vec<String>>>()
    {
        Some(permisos) => permisos,
        None => return WsResponse::error(ctx.id, 403, "Error permisos no validos"),
    };

    let new_cargo = match create_cargo(&ctx.state.pool, &nombre, &descripcion).await {
        Ok(cargo) => cargo,
        Err(err) => {
            eprintln!("[cargos_add] {err}");
            return WsResponse::error(ctx.id, 500, "error interno");
        }
    };

    WsResponse::ok(ctx.id, serde_json::json!({ "data": new_cargo }))
}
