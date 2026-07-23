use sqlx::{self, PgPool};
use uuid::Uuid;

use crate::{models::usuario::Usuario, security::password, services::error::ServiceError};


// Hash argon2 de cualquier contraseña, generado una vez con hashear()
const DUMMY_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$vBUueYcnLVdWEHGAJXkFjQ$XnRTL9GMlDT4os2lmvc2WJTFH29bXPNZtbJ8n51bw2d";

pub async fn get_usuario_username(
    pool: &PgPool,
    username: &str,
) -> Result<Option<Usuario>, ServiceError> {
    let usuario =
        sqlx::query_as::<_, Usuario>("SELECT * FROM usuarios WHERE usuario = $1 AND activo = true")
            .bind(username)
            .fetch_optional(pool)
            .await?;

    Ok(usuario)
}

pub async fn get_permisos_por_cargo(
    pool: &PgPool,
    cargo_id: Uuid,
) -> Result<Vec<String>, ServiceError> {
    let permisos = sqlx::query_scalar::<_, String>(
        r#"
        SELECT p.nombre
        FROM cargos_permisos cp
        JOIN permisos p ON p.id = cp.permiso_id
        WHERE cp.cargo_id = $1
        "#,
    )
    .bind(cargo_id)
    .fetch_all(pool)
    .await?;

    Ok(permisos)
}

pub async fn verificar_credenciales(
    pool: &PgPool,
    usuario: &str,
    password: &str,
) -> Result<Option<Usuario>, ServiceError> {
    let usuario_opt = get_usuario_username(pool, usuario).await?;
    Ok(evaluar_credenciales(usuario_opt, password))
}

fn evaluar_credenciales(usuario_opt: Option<Usuario>, password: &str) -> Option<Usuario> {
    let hash = usuario_opt
        .as_ref()
        .map(|u| u.password_hash.as_str())
        .unwrap_or(DUMMY_HASH);

    let is_correct = password::verificar(password, hash).unwrap_or(false);

    usuario_opt.filter(|_| is_correct)
}


#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn usuario_con_password(password: &str) -> Usuario {
        Usuario {
            id: Uuid::new_v4(),
            nombre: "Ana".into(),
            apellido: "Perez".into(),
            email: "ana@example.com".into(),
            usuario: "ana".into(),
            password_hash: password::hashear(password).unwrap(),
            cargo_id: Uuid::new_v4(),
            activo: true,
            creado_en: Utc::now(),
            actualizado_en: Utc::now(),
            debe_cambiar_password: false,
        }
    }

    #[test]
    fn password_correcto_devuelve_usuario() {
        let usuario = usuario_con_password("secreta123");
        let esperado = usuario.id;

        let resultado = evaluar_credenciales(Some(usuario), "secreta123");

        assert_eq!(resultado.map(|u| u.id), Some(esperado));
    }

    #[test]
    fn password_incorrecto_devuelve_none() {
        let usuario = usuario_con_password("secreta123");

        let resultado = evaluar_credenciales(Some(usuario), "otra");

        assert!(resultado.is_none());
    }

    #[test]
    fn usuario_inexistente_devuelve_none() {
        let resultado = evaluar_credenciales(None, "cualquiera");
        assert!(resultado.is_none());
    }
}