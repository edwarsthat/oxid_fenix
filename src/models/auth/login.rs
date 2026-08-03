use crate::models::validations::{ValidacionError, Validar};

#[derive(serde::Deserialize)]
pub struct LoginInput {
    pub usuario: String,
    pub password: String,
}
use serde::Deserialize;

/// El mínimo es la única regla de fuerza que imponemos: exigir símbolos y
/// mayúsculas empuja a la gente a anotar la clave al lado del monitor. El
/// máximo evita que nos manden megabytes y hashearlos sea un DoS barato.
const MIN_PASSWORD: usize = 8;
const MAX_PASSWORD: usize = 128;

#[derive(Deserialize)]
pub struct ChangePasswordInput {
    pub usuario: String,
    pub password: String,
    pub new_password: String,
}

/// Un cambio de contraseña ya validado.
pub struct CambioPassword {
    pub usuario: String,
    pub password_actual: String,
    pub password_nueva: String,
}

impl Validar for ChangePasswordInput {
    type Datos = CambioPassword;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let usuario = self.usuario.trim();

        if usuario.is_empty() {
            return Err(ValidacionError::nuevo("falta el usuario"));
        }

        let largo = self.new_password.chars().count();

        if largo < MIN_PASSWORD {
            return Err(ValidacionError::nuevo(format!(
                "la contraseña debe tener al menos {MIN_PASSWORD} caracteres"
            )));
        }

        if largo > MAX_PASSWORD {
            return Err(ValidacionError::nuevo(format!(
                "la contraseña no puede superar los {MAX_PASSWORD} caracteres"
            )));
        }

        if self.new_password == self.password {
            return Err(ValidacionError::nuevo(
                "la contraseña nueva debe ser distinta de la actual",
            ));
        }

        // Ojo: al usuario se le hace trim, a las contraseñas NO. Un espacio al
        // principio o al final es parte legítima de una contraseña.
        Ok(CambioPassword {
            usuario: usuario.to_string(),
            password_actual: self.password,
            password_nueva: self.new_password,
        })
    }
}
