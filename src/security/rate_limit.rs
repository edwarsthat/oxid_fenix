//! Rate limit por IP, con una cubeta de tokens independiente por endpoint.
//!
//! La cubeta arranca llena (`capacidad` tokens), cada petición gasta uno y el
//! tiempo la va rellenando a `por_minuto` tokens por minuto. Eso da dos números
//! con significados distintos: `capacidad` es el pico que se tolera de golpe
//! (una pantalla que dispara varias llamadas al abrirse) y `por_minuto` es el
//! ritmo sostenido que queda una vez agotado ese pico.
//!
//! El estado vive en memoria del proceso, igual que las sesiones: se pierde al
//! reiniciar y no se comparte entre instancias del backend.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock, RwLockWriteGuard};
use std::time::{Duration, Instant};

use axum::extract::{ConnectInfo, Request, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// A partir de esta cantidad de IPs vivas se intenta purgar las cubetas que ya
/// se recuperaron del todo. No se purga en cada petición: se respeta
/// [`INTERVALO_LIMPIEZA`] para no pagar un recorrido completo del mapa seguido.
const UMBRAL_LIMPIEZA: usize = 1_024;
const INTERVALO_LIMPIEZA: Duration = Duration::from_secs(60);

/// Techo duro de IPs distintas recordadas a la vez. Sin esto, un cliente con
/// muchas direcciones (un /64 de IPv6 da de sobra) hace crecer el mapa sin
/// límite hasta tumbar el proceso por memoria.
const MAX_CLAVES: usize = 50_000;

/// Espera que se le contesta a quien llega con el mapa saturado.
const ESPERA_SATURADO: Duration = Duration::from_secs(1);

/// Clave de reserva para cuando no se pudo averiguar la IP del cliente. Todas
/// esas peticiones comparten una sola cubeta a propósito: es preferible que se
/// limiten de más y no que se salten el control en silencio.
const CLAVE_DESCONOCIDA: IpAddr = IpAddr::V4(Ipv4Addr::UNSPECIFIED);

#[derive(Clone, Copy, Debug)]
struct Cubeta {
    tokens: f64,
    visto_en: Instant,
}

struct Estado {
    cubetas: HashMap<IpAddr, Cubeta>,
    ultima_limpieza: Instant,
}

#[derive(Clone)]
pub struct RateLimiter {
    capacidad: f64,
    recarga_por_seg: f64,
    inner: Arc<RwLock<Estado>>,
}

impl RateLimiter {
    /// `capacidad`: peticiones que se aceptan de golpe con la cubeta llena.
    /// `por_minuto`: ritmo sostenido al que se recupera.
    pub fn por_minuto(capacidad: u32, por_minuto: u32) -> Self {
        Self {
            capacidad: f64::from(capacidad.max(1)),
            recarga_por_seg: f64::from(por_minuto.max(1)) / 60.0,
            inner: Arc::new(RwLock::new(Estado {
                cubetas: HashMap::new(),
                ultima_limpieza: Instant::now(),
            })),
        }
    }

    /// Mismo criterio que el `SessionStore`: un panic con el lock tomado no
    /// puede dejar el rate limit muerto para siempre, porque eso convierte
    /// cualquier bug en una caída total del endpoint.
    fn escribir(&self) -> RwLockWriteGuard<'_, Estado> {
        self.inner.write().unwrap_or_else(|e| {
            Self::avisar_veneno();
            e.into_inner()
        })
    }

    fn avisar_veneno() {
        static AVISADO: AtomicBool = AtomicBool::new(false);
        if !AVISADO.swap(true, Ordering::Relaxed) {
            tracing::error!(
                "el lock del rate limit quedó envenenado: hubo un panic con el lock tomado. \
                 Se continúa operando; revisa los logs de panic."
            );
        }
    }

    fn avisar_saturado() {
        static AVISADO: AtomicBool = AtomicBool::new(false);
        if !AVISADO.swap(true, Ordering::Relaxed) {
            tracing::warn!(
                claves = MAX_CLAVES,
                "el rate limit llegó al techo de IPs recordadas: se rechazan las IPs nuevas \
                 hasta que se liberen cubetas. Suele indicar tráfico distribuido hostil."
            );
        }
    }

    /// `Ok(())` si la petición pasa; `Err(espera)` con lo que falta para que
    /// haya un token disponible.
    pub fn intentar(&self, clave: IpAddr) -> Result<(), Duration> {
        self.intentar_en(clave, Instant::now())
    }

    fn intentar_en(&self, clave: IpAddr, ahora: Instant) -> Result<(), Duration> {
        let capacidad = self.capacidad;
        let recarga = self.recarga_por_seg;
        let mut estado = self.escribir();

        if estado.cubetas.len() >= UMBRAL_LIMPIEZA
            && ahora.saturating_duration_since(estado.ultima_limpieza) >= INTERVALO_LIMPIEZA
        {
            Self::purgar(&mut estado, capacidad, recarga, ahora);
        }

        let ocupadas = estado.cubetas.len();

        match estado.cubetas.entry(clave) {
            Entry::Occupied(mut e) => {
                let cubeta = e.get_mut();
                let transcurrido = ahora
                    .saturating_duration_since(cubeta.visto_en)
                    .as_secs_f64();
                cubeta.tokens = (cubeta.tokens + transcurrido * recarga).min(capacidad);
                cubeta.visto_en = ahora;

                if cubeta.tokens >= 1.0 {
                    cubeta.tokens -= 1.0;
                    Ok(())
                } else {
                    Err(Duration::from_secs_f64((1.0 - cubeta.tokens) / recarga))
                }
            }
            Entry::Vacant(hueco) => {
                if ocupadas >= MAX_CLAVES {
                    Self::avisar_saturado();
                    return Err(ESPERA_SATURADO);
                }
                hueco.insert(Cubeta {
                    tokens: capacidad - 1.0,
                    visto_en: ahora,
                });
                Ok(())
            }
        }
    }

    /// Saca del mapa las cubetas que ya volvieron a estar llenas: olvidarlas es
    /// exactamente lo mismo que conservarlas, porque una IP nueva también
    /// arranca con la cubeta llena.
    fn purgar(estado: &mut Estado, capacidad: f64, recarga: f64, ahora: Instant) {
        estado.cubetas.retain(|_, cubeta| {
            let transcurrido = ahora
                .saturating_duration_since(cubeta.visto_en)
                .as_secs_f64();
            cubeta.tokens + transcurrido * recarga < capacidad
        });
        estado.ultima_limpieza = ahora;
    }

    #[cfg(test)]
    fn total(&self) -> usize {
        self.escribir().cubetas.len()
    }
}

/// Middleware de axum: se monta por ruta con
/// `axum::middleware::from_fn_with_state(limiter, limitar)`.
///
/// La IP se saca a mano de las extensiones en vez de con el extractor
/// `ConnectInfo`: si el router no se sirviera con
/// `into_make_service_with_connect_info`, el extractor devolvería 500 y dejaría
/// el endpoint caído. Así sigue funcionando, limitado bajo
/// [`CLAVE_DESCONOCIDA`], y el aviso queda en los logs.
pub async fn limitar(State(limiter): State<RateLimiter>, req: Request, next: Next) -> Response {
    let clave = match req.extensions().get::<ConnectInfo<SocketAddr>>() {
        // Solo la IP: el puerto de origen cambia en cada conexión TCP, así que
        // meterlo en la clave le daría a un mismo cliente cubetas infinitas.
        Some(ConnectInfo(addr)) => addr.ip(),
        None => {
            avisar_sin_connect_info();
            CLAVE_DESCONOCIDA
        }
    };

    match limiter.intentar(clave) {
        Ok(()) => next.run(req).await,
        Err(espera) => respuesta_429(espera),
    }
}

fn avisar_sin_connect_info() {
    static AVISADO: AtomicBool = AtomicBool::new(false);
    if !AVISADO.swap(true, Ordering::Relaxed) {
        tracing::error!(
            "el rate limit no pudo leer la IP del cliente: falta servir el router con \
             into_make_service_with_connect_info. Se limita a todos bajo una única cubeta."
        );
    }
}

fn respuesta_429(espera: Duration) -> Response {
    let segundos = espera.as_secs_f64().ceil().max(1.0) as u64;

    let mut resp = (
        StatusCode::TOO_MANY_REQUESTS,
        "demasiadas solicitudes".to_string(),
    )
        .into_response();

    // Retry-After va en segundos enteros (RFC 9110 §10.2.3); el cliente sabe
    // cuánto esperar en vez de reintentar a ciegas.
    if let Ok(valor) = HeaderValue::from_str(&segundos.to_string()) {
        resp.headers_mut().insert(header::RETRY_AFTER, valor);
    }

    resp
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(ultimo: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(10, 0, 0, ultimo))
    }

    #[test]
    fn permite_hasta_la_capacidad_y_despues_rechaza() {
        let limiter = RateLimiter::por_minuto(3, 60);
        let ahora = Instant::now();

        for i in 0..3 {
            assert!(
                limiter.intentar_en(ip(1), ahora).is_ok(),
                "la petición {i} debería entrar en el pico"
            );
        }

        assert!(limiter.intentar_en(ip(1), ahora).is_err());
    }

    #[test]
    fn el_tiempo_recarga_la_cubeta() {
        // 60 por minuto = 1 token por segundo
        let limiter = RateLimiter::por_minuto(2, 60);
        let t0 = Instant::now();

        assert!(limiter.intentar_en(ip(1), t0).is_ok());
        assert!(limiter.intentar_en(ip(1), t0).is_ok());
        assert!(limiter.intentar_en(ip(1), t0).is_err());

        // un segundo después hay exactamente un token más
        let t1 = t0 + Duration::from_secs(1);
        assert!(limiter.intentar_en(ip(1), t1).is_ok());
        assert!(limiter.intentar_en(ip(1), t1).is_err());
    }

    #[test]
    fn la_recarga_no_pasa_de_la_capacidad() {
        let limiter = RateLimiter::por_minuto(2, 60);
        let t0 = Instant::now();

        assert!(limiter.intentar_en(ip(1), t0).is_ok());

        // una hora parada no acumula más tokens que la capacidad
        let t1 = t0 + Duration::from_secs(3_600);
        assert!(limiter.intentar_en(ip(1), t1).is_ok());
        assert!(limiter.intentar_en(ip(1), t1).is_ok());
        assert!(limiter.intentar_en(ip(1), t1).is_err());
    }

    #[test]
    fn cada_ip_tiene_su_propia_cubeta() {
        let limiter = RateLimiter::por_minuto(1, 60);
        let ahora = Instant::now();

        assert!(limiter.intentar_en(ip(1), ahora).is_ok());
        assert!(limiter.intentar_en(ip(1), ahora).is_err());

        // que una IP se pase no puede dejar fuera a las demás
        assert!(limiter.intentar_en(ip(2), ahora).is_ok());
    }

    #[test]
    fn la_espera_devuelta_alcanza_para_el_siguiente_token() {
        let limiter = RateLimiter::por_minuto(1, 12); // 1 token cada 5 s
        let t0 = Instant::now();

        assert!(limiter.intentar_en(ip(1), t0).is_ok());
        let espera = limiter.intentar_en(ip(1), t0).unwrap_err();

        assert_eq!(espera, Duration::from_secs(5));
        assert!(limiter.intentar_en(ip(1), t0 + espera).is_ok());
    }

    #[test]
    fn dos_limiters_no_comparten_estado() {
        // cada endpoint monta el suyo: gastar el de /login no toca el de /health
        let login = RateLimiter::por_minuto(1, 60);
        let health = RateLimiter::por_minuto(1, 60);
        let ahora = Instant::now();

        assert!(login.intentar_en(ip(1), ahora).is_ok());
        assert!(login.intentar_en(ip(1), ahora).is_err());
        assert!(health.intentar_en(ip(1), ahora).is_ok());
    }

    #[test]
    fn los_clones_comparten_el_mismo_estado() {
        // el middleware clona el limiter en cada petición: si el clon no
        // compartiera el mapa, el límite no contaría nada
        let limiter = RateLimiter::por_minuto(1, 60);
        let clon = limiter.clone();
        let ahora = Instant::now();

        assert!(limiter.intentar_en(ip(1), ahora).is_ok());
        assert!(clon.intentar_en(ip(1), ahora).is_err());
    }

    #[test]
    fn purgar_borra_las_cubetas_llenas_y_deja_las_gastadas() {
        let limiter = RateLimiter::por_minuto(2, 60);
        let t0 = Instant::now();

        limiter.intentar_en(ip(1), t0).unwrap(); // le queda 1 token
        limiter.intentar_en(ip(2), t0).unwrap();
        limiter.intentar_en(ip(2), t0).unwrap(); // se quedó sin tokens
        assert_eq!(limiter.total(), 2);

        // a 1 token por segundo, al segundo la 1 vuelve a estar llena (2 de 2)
        // y la 2 sigue a medias (1 de 2)
        let t1 = t0 + Duration::from_secs(1);
        {
            let mut estado = limiter.escribir();
            RateLimiter::purgar(&mut estado, limiter.capacidad, limiter.recarga_por_seg, t1);
        }

        assert_eq!(limiter.total(), 1, "solo debe quedar la cubeta a medias");

        // y la que quedó conserva su deuda: le queda 1 token, no los 2 de una
        // cubeta recién estrenada
        assert!(limiter.intentar_en(ip(2), t1).is_ok());
        assert!(limiter.intentar_en(ip(2), t1).is_err());
    }

    #[test]
    fn olvidar_una_cubeta_llena_no_regala_peticiones() {
        // purgar solo saca cubetas llenas, y una IP nueva arranca llena:
        // el resultado tiene que ser idéntico a no haber purgado
        let limiter = RateLimiter::por_minuto(2, 60);
        let t0 = Instant::now();

        limiter.intentar_en(ip(1), t0).unwrap();
        let t1 = t0 + Duration::from_secs(60);
        {
            let mut estado = limiter.escribir();
            RateLimiter::purgar(&mut estado, limiter.capacidad, limiter.recarga_por_seg, t1);
        }
        assert_eq!(limiter.total(), 0);

        assert!(limiter.intentar_en(ip(1), t1).is_ok());
        assert!(limiter.intentar_en(ip(1), t1).is_ok());
        assert!(limiter.intentar_en(ip(1), t1).is_err());
    }

    #[test]
    fn sigue_operativo_tras_un_panico_con_el_lock_tomado() {
        let limiter = RateLimiter::por_minuto(2, 60);
        let clon = limiter.clone();

        let anterior = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {})); // silencia el backtrace esperado
        let _ = std::thread::spawn(move || {
            let _guard = clon.inner.write().unwrap();
            panic!("envenena el lock");
        })
        .join();
        std::panic::set_hook(anterior);

        assert!(limiter.intentar(ip(1)).is_ok());
    }

    // ── el middleware montado sobre un Router ───────

    fn router_de_prueba(limiter: RateLimiter) -> axum::Router {
        axum::Router::new()
            .route("/x", axum::routing::get(|| async { "ok" }))
            .layer(axum::middleware::from_fn_with_state(limiter, limitar))
    }

    fn peticion(desde: Option<SocketAddr>) -> Request {
        let mut req = Request::builder()
            .uri("/x")
            .body(axum::body::Body::empty())
            .unwrap();
        if let Some(addr) = desde {
            req.extensions_mut().insert(ConnectInfo(addr));
        }
        req
    }

    async fn pedir(router: &axum::Router, desde: Option<SocketAddr>) -> Response {
        use tower::ServiceExt;
        router.clone().oneshot(peticion(desde)).await.unwrap()
    }

    #[tokio::test]
    async fn el_middleware_deja_pasar_y_luego_devuelve_429() {
        let router = router_de_prueba(RateLimiter::por_minuto(2, 1));
        let cliente: SocketAddr = "203.0.113.7:5555".parse().unwrap();

        assert_eq!(pedir(&router, Some(cliente)).await.status(), StatusCode::OK);
        assert_eq!(pedir(&router, Some(cliente)).await.status(), StatusCode::OK);

        let resp = pedir(&router, Some(cliente)).await;
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert!(resp.headers().contains_key(header::RETRY_AFTER));
    }

    #[tokio::test]
    async fn el_middleware_separa_por_ip_y_no_mira_el_puerto() {
        let router = router_de_prueba(RateLimiter::por_minuto(1, 1));
        let cliente: SocketAddr = "203.0.113.7:5555".parse().unwrap();
        // mismo host, otro puerto de origen: cada conexión TCP estrena puerto,
        // así que si contara, el límite no serviría absolutamente para nada
        let mismo_host_otro_puerto: SocketAddr = "203.0.113.7:6666".parse().unwrap();
        let otro_host: SocketAddr = "203.0.113.8:5555".parse().unwrap();

        assert_eq!(pedir(&router, Some(cliente)).await.status(), StatusCode::OK);
        assert_eq!(
            pedir(&router, Some(mismo_host_otro_puerto)).await.status(),
            StatusCode::TOO_MANY_REQUESTS
        );
        assert_eq!(
            pedir(&router, Some(otro_host)).await.status(),
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn sin_connect_info_limita_igual_en_vez_de_reventar() {
        let router = router_de_prueba(RateLimiter::por_minuto(1, 1));

        assert_eq!(pedir(&router, None).await.status(), StatusCode::OK);
        assert_eq!(
            pedir(&router, None).await.status(),
            StatusCode::TOO_MANY_REQUESTS,
            "sin IP se limita bajo una cubeta común, nunca con un 500"
        );
    }

    #[test]
    fn respuesta_429_lleva_retry_after_redondeado_hacia_arriba() {
        let resp = respuesta_429(Duration::from_millis(1_200));

        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(
            resp.headers().get(header::RETRY_AFTER).unwrap(),
            "2",
            "1.2 s se redondea a 2: con 1 el cliente reintentaría demasiado pronto"
        );
    }

    #[test]
    fn respuesta_429_nunca_dice_retry_after_cero() {
        let resp = respuesta_429(Duration::from_millis(1));

        assert_eq!(resp.headers().get(header::RETRY_AFTER).unwrap(), "1");
    }
}
