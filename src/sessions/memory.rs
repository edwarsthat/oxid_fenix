use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::sessions::error::SessionError;

#[derive(Clone, Debug)]
pub struct Session {
    pub usuario_id: Uuid,
    pub cargo_id: Uuid,
    pub expira_en: DateTime<Utc>,
    pub permisos: Arc<HashSet<String>>
}

#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<RwLock<HashMap<Uuid, Session>>>,

}

impl SessionStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn crear(
        &self,
        usuario_id: Uuid,
        cargo_id: Uuid,
        duracion: Duration,
        permisos: Arc<HashSet<String>>,
    ) -> Result<Uuid, SessionError> {
        let id = Uuid::new_v4();
        let session = Session {
            usuario_id,
            cargo_id,
            expira_en: Utc::now() + duracion,
            permisos,
        };

        let mut mapa = self
            .inner
            .write()
            .map_err(|_| SessionError::LockEnvenenado)?;

        mapa.insert(id, session);
        Ok(id)
    }

    pub fn validar(&self, id: &Uuid) -> Result<Option<Session>, SessionError> {
        let mut mapa = self
            .inner
            .write()
            .map_err(|_| SessionError::LockEnvenenado)?;

        match mapa.get(id) {
            Some(s) if s.expira_en > Utc::now() => Ok(Some(s.clone())),
            Some(_) => {
                mapa.remove(id); // expirada → la eliminamos
                Ok(None)
            }
            None => Ok(None),
        }
    }

    pub fn eliminar(&self, id: &Uuid) -> Result<(), SessionError> {
        self.inner
            .write()
            .map_err(|_| SessionError::LockEnvenenado)?
            .remove(id);
        Ok(())
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    fn permisos(items: &[&str]) -> Arc<HashSet<String>> {
        Arc::new(items.iter().map(|p| p.to_string()).collect())
    }

    #[test]
    fn crear_genera_sesion_valiida() {
        let store = SessionStore::new();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();
        let permisos = permisos(&["usuarios.leer", "usuarios.crear"]);

        let id = store
            .crear(usuario_id, cargo_id, Duration::hours(1), permisos.clone())
            .expect("crear deberia devolver un Uuid");

        let session = store
            .validar(&id)
            .expect("validar no deberia fallar")
            .expect("la sesion deberia existir y no estar expirada");

        assert_eq!(session.usuario_id, usuario_id);
        assert_eq!(session.cargo_id, cargo_id);
        assert_eq!(session.permisos, permisos);
        assert!(session.expira_en > Utc::now());
    }

    #[test]
    fn crear_session_expirada() {
        let store = SessionStore::new();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();

        let id = store
            .crear(
                usuario_id,
                cargo_id,
                Duration::seconds(-1),
                permisos(&["usuarios.leer"]),
            )
            .unwrap();

        let session = store.validar(&id).expect("validar no deberia fallar");

        assert!(session.is_none());
    }

    #[test]
    fn eliminar_sesion() {
        let store = SessionStore::new();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();

        let id = store
            .crear(
                usuario_id,
                cargo_id,
                Duration::hours(1),
                permisos(&["usuarios.leer"]),
            )
            .expect("no deberia fallar");

        store.eliminar(&id).expect("Deberia borrar sin problema");

        let session = store.validar(&id).unwrap();

        assert!(session.is_none());
    }

    #[test]
    fn validar_id_inexistente() {
        let store = SessionStore::new();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();
        let wrong_id = Uuid::new_v4();

        store
            .crear(
                usuario_id,
                cargo_id,
                Duration::hours(1),
                permisos(&["usuarios.leer"]),
            )
            .unwrap();

        let session = store.validar(&wrong_id).unwrap();

        assert!(session.is_none());
    }

    #[test]
    fn eliminar_id_inexistente_no_falla() {
        let store = SessionStore::new();
        let id = Uuid::new_v4();

        store
            .eliminar(&id)
            .expect("eliminar deberia ser idempotente");
    }

    #[test]
    fn clones_comparten_el_mismo_estado(){
        let store = SessionStore::new();
        let store2 = store.clone();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();

        let id = store
            .crear(
                usuario_id,
                cargo_id,
                Duration::hours(1),
                permisos(&["usuarios.leer"]),
            )
            .unwrap();
        assert!(store2.validar(&id).unwrap().is_some());

        store2.eliminar(&id).unwrap();
        assert!(store.validar(&id).unwrap().is_none());
    }
}
