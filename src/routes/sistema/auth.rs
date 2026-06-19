use crate::controller::sistema::auth as controller;
use crate::routes::protocol::{Ctx, WsResponse};

/// Router del dominio `auth` (dentro del área `sistema`).
/// resto: "usuario::listar"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    println!("[sistema::auth] ruta recibida: '{resto}' (id={})", ctx.id);

    match resto {
        "usuario::listar" => controller::list_usuarios(ctx).await,
        _ => {
            println!("[sistema::auth] !! acción desconocida: '{resto}'");
            WsResponse::error(ctx.id, 404, "Acción desconocida")
        }
    }
}
