use crate::controller::proceso as controller;
use crate::routes::protocol::{Ctx, WsResponse};

/// Router del área `proceso`: lo que pasa con la fruta una vez que entró y hasta
/// que sale. Hoy solo la programación —qué lote está montado en la línea—, que
/// es de donde cada pesada de la báscula saca su lote.
/// resto: "programaciones_proceso:add"
pub async fn route(resto: &str, ctx: Ctx) -> WsResponse {
    match resto {
        "programaciones_proceso:add" => {
            if !ctx.permisos.contains("programaciones_proceso:add") {
                return WsResponse::error(ctx.id, 403, "sin permiso");
            }
            controller::programacion_proceso::programacion_proceso_add(ctx).await
        }
        _ => WsResponse::error(ctx.id, 404, "Acción desconocida"),
    }
}
