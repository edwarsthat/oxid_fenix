
use argon2::{
    Argon2, PasswordHasher, PasswordVerifier, password_hash::{PasswordHash, SaltString, rand_core::OsRng}
};

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
}