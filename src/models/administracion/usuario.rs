use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{ValidacionError, Validar, uuid_obligatorio, uuid_requerido};

#[derive(Debug, FromRow, Serialize)]
pub struct Usuario {
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub usuario: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub cargo_id: Uuid,
    pub activo: bool,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
    pub debe_cambiar_password: bool,
}

#[derive(Debug, FromRow, Serialize)]
pub struct UsuarioListItem {
    pub id: Uuid,
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub usuario: String,
    pub cargo_id: Uuid,
    pub activo: bool,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
    pub debe_cambiar_password: bool,
}

#[derive(Debug, Deserialize)]
pub struct UsuariosAddPayload {
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub usuario: String,
    pub cargo_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UsuariosUpdatePayload {
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub usuario: String,
    pub cargo_id: String,
    pub usuario_id: String,
}

/// Campos de usuario ya normalizados (trim, email en minúsculas) y validados.
/// Es lo único que los controllers le pasan a la capa de servicios: si tenés un
/// `UsuarioDatos`, los datos ya son válidos.
#[derive(Debug)]
pub struct UsuarioDatos {
    pub nombre: String,
    pub apellido: String,
    pub email: String,
    pub usuario: String,
    pub cargo_id: Uuid,
}

/// Largos según la migración `crear_usuarios`: pasarse hace fallar el INSERT con
/// un 22001 crudo, que el cliente vería como 500. Mejor cortarlo acá con un 400.
const MAX_NOMBRE: usize = 100;
const MAX_EMAIL: usize = 255;

fn normalizar(
    nombre: &str,
    apellido: &str,
    email: &str,
    usuario: &str,
    cargo_id: &str,
) -> Result<UsuarioDatos, ValidacionError> {
    let nombre = nombre.trim();
    let apellido = apellido.trim();
    let email = email.trim().to_lowercase();
    let usuario = usuario.trim();

    if nombre.is_empty() || apellido.is_empty() || email.is_empty() || usuario.is_empty() {
        return Err(ValidacionError::nuevo("hay campos vacios"));
    }

    if nombre.chars().count() > MAX_NOMBRE || apellido.chars().count() > MAX_NOMBRE {
        return Err(ValidacionError::nuevo(format!(
            "nombre y apellido no pueden superar {MAX_NOMBRE} caracteres"
        )));
    }

    if email.chars().count() > MAX_EMAIL || usuario.chars().count() > MAX_EMAIL {
        return Err(ValidacionError::nuevo(format!(
            "email y usuario no pueden superar {MAX_EMAIL} caracteres"
        )));
    }

    if !es_email_valido(&email) {
        return Err(ValidacionError::nuevo("email no valido"));
    }

    if usuario.contains(char::is_whitespace) {
        return Err(ValidacionError::nuevo("el usuario no puede tener espacios"));
    }

    let cargo_id = uuid_obligatorio(cargo_id, "cargo_id")?;

    Ok(UsuarioDatos {
        nombre: nombre.to_string(),
        apellido: apellido.to_string(),
        email,
        usuario: usuario.to_string(),
        cargo_id,
    })
}

/// Chequeo mínimo: un `@`, algo a cada lado y un punto en el dominio. No intenta
/// ser un validador de RFC 5322, solo atajar dedazos.
fn es_email_valido(email: &str) -> bool {
    let Some((local, dominio)) = email.split_once('@') else {
        return false;
    };

    !local.is_empty()
        && !dominio.is_empty()
        && !dominio.contains('@')
        && dominio.contains('.')
        && !dominio.starts_with('.')
        && !dominio.ends_with('.')
        && !email.contains(char::is_whitespace)
}

/// Payload de las operaciones que solo señalan a un usuario (eliminar, activar,
/// reiniciar contraseña, cerrarle las sesiones).
#[derive(Debug, Deserialize)]
pub struct UsuarioIdPayload {
    pub usuario_id: Option<String>,
}

impl Validar for UsuarioIdPayload {
    type Datos = Uuid;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        uuid_requerido(self.usuario_id, "usuario_id")
    }
}

impl UsuariosAddPayload {
    pub fn validar(&self) -> Result<UsuarioDatos, ValidacionError> {
        normalizar(
            &self.nombre,
            &self.apellido,
            &self.email,
            &self.usuario,
            &self.cargo_id,
        )
    }
}

impl UsuariosUpdatePayload {
    /// Devuelve los datos normalizados y el id del usuario a modificar.
    pub fn validar(&self) -> Result<(UsuarioDatos, Uuid), ValidacionError> {
        let datos = normalizar(
            &self.nombre,
            &self.apellido,
            &self.email,
            &self.usuario,
            &self.cargo_id,
        )?;

        let usuario_id = uuid_obligatorio(&self.usuario_id, "usuario_id")?;

        Ok((datos, usuario_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn add_payload() -> UsuariosAddPayload {
        UsuariosAddPayload {
            nombre: "Ana".into(),
            apellido: "Perez".into(),
            email: "ana@example.com".into(),
            usuario: "ana".into(),
            cargo_id: Uuid::new_v4().to_string(),
        }
    }

    #[test]
    fn validar_recorta_espacios_y_baja_el_email() {
        let payload = UsuariosAddPayload {
            nombre: "  Ana  ".into(),
            apellido: " Perez ".into(),
            email: "  Ana@Example.COM ".into(),
            usuario: " ana ".into(),
            ..add_payload()
        };

        let datos = payload.validar().expect("deberia ser valido");

        assert_eq!(datos.nombre, "Ana");
        assert_eq!(datos.apellido, "Perez");
        assert_eq!(datos.email, "ana@example.com");
        assert_eq!(datos.usuario, "ana");
    }

    #[test]
    fn validar_rechaza_campos_vacios() {
        let payload = UsuariosAddPayload {
            nombre: "   ".into(),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "hay campos vacios"
        );
    }

    #[test]
    fn validar_rechaza_email_sin_arroba() {
        let payload = UsuariosAddPayload {
            email: "ana.example.com".into(),
            ..add_payload()
        };

        assert_eq!(payload.validar().unwrap_err().mensaje(), "email no valido");
    }

    #[test]
    fn validar_rechaza_email_sin_punto_en_el_dominio() {
        let payload = UsuariosAddPayload {
            email: "ana@example".into(),
            ..add_payload()
        };

        assert_eq!(payload.validar().unwrap_err().mensaje(), "email no valido");
    }

    #[test]
    fn validar_rechaza_usuario_con_espacios() {
        let payload = UsuariosAddPayload {
            usuario: "ana perez".into(),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el usuario no puede tener espacios"
        );
    }

    #[test]
    fn validar_rechaza_nombre_demasiado_largo() {
        let payload = UsuariosAddPayload {
            nombre: "a".repeat(MAX_NOMBRE + 1),
            ..add_payload()
        };

        assert!(
            payload
                .validar()
                .unwrap_err()
                .mensaje()
                .contains("100 caracteres")
        );
    }

    #[test]
    fn validar_rechaza_cargo_id_invalido() {
        let payload = UsuariosAddPayload {
            cargo_id: "no-es-un-uuid".into(),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el cargo_id no es un UUID válido"
        );
    }

    #[test]
    fn validar_update_devuelve_los_datos_y_el_usuario_id() {
        let usuario_id = Uuid::new_v4();
        let payload = UsuariosUpdatePayload {
            nombre: " Ana ".into(),
            apellido: "Perez".into(),
            email: "ANA@EXAMPLE.COM".into(),
            usuario: "ana".into(),
            cargo_id: Uuid::new_v4().to_string(),
            usuario_id: usuario_id.to_string(),
        };

        let (datos, id) = payload.validar().expect("deberia ser valido");

        assert_eq!(datos.nombre, "Ana");
        assert_eq!(datos.email, "ana@example.com");
        assert_eq!(id, usuario_id);
    }

    #[test]
    fn validar_update_rechaza_usuario_id_invalido() {
        let payload = UsuariosUpdatePayload {
            nombre: "Ana".into(),
            apellido: "Perez".into(),
            email: "ana@example.com".into(),
            usuario: "ana".into(),
            cargo_id: Uuid::new_v4().to_string(),
            usuario_id: "no-es-un-uuid".into(),
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el usuario_id no es un UUID válido"
        );
    }
}
