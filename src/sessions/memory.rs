use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Duration, Utc};
use uuid::Uuid;

use crate::sessions::error::SessionError;

#[derive(Clone, Debug)]
pub struct Session {
    pub usuario_id: Uuid,
    pub cargo_id: Uuid,
    pub expira_en: DateTime<Utc>,
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
    ) -> Result<Uuid, SessionError> {
        let id = Uuid::new_v4();
        let session = Session {
            usuario_id,
            cargo_id,
            expira_en: Utc::now() + duracion,
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

    pub fn eliminar(&self, id:&Uuid) -> Result<(), SessionError> {
        self.inner
            .write()
            .map_err(|_| SessionError::LockEnvenenado)?
            .remove(id);
        Ok(())
    }
}
