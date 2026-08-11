use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
pub struct Session {
    pub usuario_id: Uuid,
    pub cargo_id: Uuid,
    pub expira_en: DateTime<Utc>,
    pub permisos: Arc<HashSet<String>>,
    pub debe_cambiar_password: bool,
}

#[derive(Clone, Default)]
pub struct SessionStore {
    inner: Arc<RwLock<HashMap<Uuid, Session>>>,
}

impl SessionStore {
    fn leer(&self) -> RwLockReadGuard<'_, HashMap<Uuid, Session>> {
        self.inner.read().unwrap_or_else(|e| {
            Self::avisar_veneno();
            e.into_inner()
        })
    }

    fn escribir(&self) -> RwLockWriteGuard<'_, HashMap<Uuid, Session>> {
        self.inner.write().unwrap_or_else(|e| {
            Self::avisar_veneno();
            e.into_inner()
        })
    }

    fn avisar_veneno() {
        static AVISADO: AtomicBool = AtomicBool::new(false);
        if !AVISADO.swap(true, Ordering::Relaxed) {
            tracing::error!(
                "el lock de sesiones quedó envenenado: hubo un panic con el lock tomado. \
                 Se continúa operando; revisa los logs de panic."
            );
        }
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn crear(
        &self,
        usuario_id: Uuid,
        cargo_id: Uuid,
        duracion: Duration,
        permisos: Arc<HashSet<String>>,
        debe_cambiar_password: bool,
    ) -> Uuid {
        let id = Uuid::new_v4();
        let session = Session {
            usuario_id,
            cargo_id,
            expira_en: Utc::now() + duracion,
            permisos,
            debe_cambiar_password,
        };

        self.escribir().insert(id, session);
        id
    }

    pub fn validar(&self, id: &Uuid) -> Option<Session> {
        let mapa = self.leer();
        match mapa.get(id) {
            Some(s) if s.expira_en > Utc::now() => Some(s.clone()),
            _ => None,
        }
    }

    pub fn actualizar_permisos_por_usuario(
        &self,
        usuario_id: Uuid,
        cargo_id: Uuid,
        permisos: Arc<HashSet<String>>,
    ) -> usize {
        let mut mapa = self.escribir();
        let mut n = 0;
        // Nada falible aquí adentro: solo asignaciones y Arc::clone.
        for s in mapa.values_mut().filter(|s| s.usuario_id == usuario_id) {
            s.cargo_id = cargo_id;
            s.permisos = permisos.clone();
            n += 1;
        }
        n
    }

    pub fn eliminar(&self, id: &Uuid) {
        self.escribir().remove(id);
    }

    pub fn eliminar_por_usuario(&self, usuario_id: Uuid) -> usize {
        let mut mapa = self.escribir();
        let antes = mapa.len();
        mapa.retain(|_, s| s.usuario_id != usuario_id);
        antes - mapa.len()
    }

    pub fn eliminar_por_cargo(&self, cargo_id: Uuid) -> usize {
        let mut mapa = self.escribir();
        let antes = mapa.len();
        mapa.retain(|_, s| s.cargo_id != cargo_id);
        antes - mapa.len()
    }

    pub fn limpiar_expiradas(&self) -> usize {
        let ahora = Utc::now();
        let mut mapa = self.escribir();
        let antes = mapa.len();
        mapa.retain(|_, s| s.expira_en > ahora);
        antes - mapa.len()
    }

    pub fn iniciar_limpieza(&self, cada: std::time::Duration) {
        let store = self.clone(); // Clone barato: es un Arc
        tokio::spawn(async move {
            let mut tick = tokio::time::interval(cada);
            tick.tick().await; // el primer tick dispara de inmediato; lo descartamos

            loop {
                tick.tick().await;
                let n = store.limpiar_expiradas();
                if n > 0 {
                    tracing::debug!(sesiones = n, "sesiones expiradas eliminadas");
                }
            }
        });
    }

    pub fn total(&self) -> usize {
        self.leer().len()
    }

    /// Todas las sesiones vivas, de la más lejana a expirar a la más próxima.
    ///
    /// No devuelve la clave del mapa a propósito: esa clave es el token de
    /// sesión, y exponerla en un listado permitiría suplantar a cualquier
    /// usuario conectado. Para cerrar sesiones se usa [`Self::eliminar_por_usuario`].
    pub fn listar(&self) -> Vec<Session> {
        let ahora = Utc::now();
        let mut sesiones: Vec<Session> = self
            .leer()
            .values()
            .filter(|s| s.expira_en > ahora)
            .cloned()
            .collect();
        sesiones.sort_unstable_by_key(|s| std::cmp::Reverse(s.expira_en));
        sesiones
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

        let id = store.crear(
            usuario_id,
            cargo_id,
            Duration::hours(1),
            permisos.clone(),
            false,
        );

        let session = store
            .validar(&id)
            .expect("la sesion deberia existir y no estar expirada");

        assert_eq!(session.usuario_id, usuario_id);
        assert_eq!(session.cargo_id, cargo_id);
        assert_eq!(session.permisos, permisos);
        assert!(session.expira_en > Utc::now());
        assert!(!session.debe_cambiar_password);
    }

    #[test]
    fn crear_conserva_debe_cambiar_password() {
        let store = SessionStore::new();

        let id = store.crear(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Duration::hours(1),
            permisos(&["usuarios.leer"]),
            true,
        );

        let session = store.validar(&id).expect("la sesion existe");

        assert!(session.debe_cambiar_password);
    }

    #[test]
    fn crear_session_expirada() {
        let store = SessionStore::new();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();

        let id = store.crear(
            usuario_id,
            cargo_id,
            Duration::seconds(-1),
            permisos(&["usuarios.leer"]),
            false,
        );

        assert!(store.validar(&id).is_none());
    }

    #[test]
    fn eliminar_sesion() {
        let store = SessionStore::new();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();

        let id = store.crear(
            usuario_id,
            cargo_id,
            Duration::hours(1),
            permisos(&["usuarios.leer"]),
            false,
        );

        store.eliminar(&id);

        assert!(store.validar(&id).is_none());
    }

    #[test]
    fn validar_id_inexistente() {
        let store = SessionStore::new();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();
        let wrong_id = Uuid::new_v4();

        store.crear(
            usuario_id,
            cargo_id,
            Duration::hours(1),
            permisos(&["usuarios.leer"]),
            false,
        );

        assert!(store.validar(&wrong_id).is_none());
    }

    #[test]
    fn eliminar_id_inexistente_no_falla() {
        let store = SessionStore::new();
        let id = Uuid::new_v4();

        // eliminar es idempotente: borrar algo que no existe no revienta
        store.eliminar(&id);

        assert_eq!(store.total(), 0);
    }

    #[test]
    fn clones_comparten_el_mismo_estado() {
        let store = SessionStore::new();
        let store2 = store.clone();
        let usuario_id = Uuid::new_v4();
        let cargo_id = Uuid::new_v4();

        let id = store.crear(
            usuario_id,
            cargo_id,
            Duration::hours(1),
            permisos(&["usuarios.leer"]),
            false,
        );
        assert!(store2.validar(&id).is_some());

        store2.eliminar(&id);
        assert!(store.validar(&id).is_none());
    }

    #[test]
    fn el_store_sigue_operativo_tras_un_panico_con_el_lock_tomado() {
        let store = SessionStore::new();
        let store2 = store.clone();

        // envenena el lock: pánico con el guard de escritura tomado
        let anterior = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // silencia el backtrace esperado
        let _ = std::thread::spawn(move || {
            let _guard = store2.inner.write().unwrap();
            panic!("envenena el lock");
        })
        .join();
        std::panic::set_hook(anterior);

        // antes de recuperar el guard, todo lo de abajo fallaba para siempre
        let id = store.crear(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Duration::hours(1),
            permisos(&["usuarios.leer"]),
            false,
        );

        assert!(store.validar(&id).is_some());
        assert_eq!(store.eliminar_por_cargo(Uuid::new_v4()), 0);
        assert!(store.validar(&id).is_some());
    }

    #[test]
    fn listar_omite_las_expiradas_y_ordena_por_vencimiento() {
        let store = SessionStore::new();
        let lejana = Uuid::new_v4();
        let cercana = Uuid::new_v4();

        store.crear(
            cercana,
            Uuid::new_v4(),
            Duration::hours(1),
            permisos(&[]),
            false,
        );
        store.crear(
            lejana,
            Uuid::new_v4(),
            Duration::hours(5),
            permisos(&[]),
            false,
        );
        store.crear(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Duration::seconds(-1),
            permisos(&[]),
            false,
        );

        let sesiones = store.listar();

        assert_eq!(sesiones.len(), 2, "la vencida no debe aparecer");
        assert_eq!(sesiones[0].usuario_id, lejana);
        assert_eq!(sesiones[1].usuario_id, cercana);
    }

    #[test]
    fn listar_sin_sesiones_devuelve_vacio() {
        assert!(SessionStore::new().listar().is_empty());
    }

    #[test]
    fn limpiar_expiradas_borra_solo_las_vencidas() {
        let store = SessionStore::new();

        let viva = store.crear(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Duration::hours(1),
            permisos(&[]),
            false,
        );
        let vencida = store.crear(
            Uuid::new_v4(),
            Uuid::new_v4(),
            Duration::seconds(-1),
            permisos(&[]),
            false,
        );

        assert_eq!(store.total(), 2);
        assert_eq!(store.limpiar_expiradas(), 1);
        assert_eq!(store.total(), 1);
        assert!(store.validar(&viva).is_some());
        assert!(store.validar(&vencida).is_none());
    }
}
