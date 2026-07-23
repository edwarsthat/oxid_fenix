
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier, password_hash::{PasswordHash, SaltString, rand_core::{OsRng, RngCore}}
};

const CHARSET_TEMPORAL: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz23456789";
const LARGO_TEMPORAL: usize = 6;

pub fn hashear(password: &str) -> Result<String, argon2::password_hash::Error> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2.hash_password(password.as_bytes(), &salt)?;
    Ok(hash.to_string())
}

pub fn verificar(password: &str, hash_guardado: &str) -> Result<bool,  argon2::password_hash::Error> {
    let hash = PasswordHash::new(hash_guardado)?;
    match Argon2::default().verify_password(password.as_bytes(), &hash) {
        Ok(()) => Ok(true),
        Err(argon2::password_hash::Error::Password) => Ok(false),
        Err(e) => Err(e)
    }
}

/// Password temporal para usuarios nuevos (quedan con debe_cambiar_password = true).
pub fn generar_temporal() -> String {
    let mut rng = OsRng;
    (0..LARGO_TEMPORAL)
        .map(|_| {
            let idx = (rng.next_u32() as usize) % CHARSET_TEMPORAL.len();
            CHARSET_TEMPORAL[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_and_verify_coninciden(){
        let hash = hashear("secreta123").unwrap();
        assert!(verificar("secreta123", &hash).unwrap());
    }

    #[test]
    fn password_incorrect() {
        let hash = hashear("secreta123").unwrap();
        assert!(!verificar("otra", &hash).unwrap());

    }

    #[test]
    fn generar_temporal_tiene_el_largo_esperado_y_charset_valido() {
        let temporal = generar_temporal();

        assert_eq!(temporal.len(), LARGO_TEMPORAL);
        assert!(temporal.bytes().all(|b| CHARSET_TEMPORAL.contains(&b)));
    }

    #[test]
    fn generar_temporal_no_repite_siempre_el_mismo_valor() {
        let a = generar_temporal();
        let b = generar_temporal();

        assert_ne!(a, b);
    }
}