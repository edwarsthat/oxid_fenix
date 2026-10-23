use crate::{app::error::ApiError, services::error::ServiceError};

fn rechazo_o_interno(err: ServiceError) -> ApiError {
    match err {
        ServiceError::NotFound(msg)
        | ServiceError::Conflict(msg)
        | ServiceError::BadRequest(msg) => ApiError::PesajeRechazado(msg),
        otro => ApiError::Service(otro),
    }
}

pub async fn ingreso_movimiento_materia_prima() {
    todo!()
}
