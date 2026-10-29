use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::inventarios::llaves_nfc::uid_nfc;
use crate::models::validations::{Rango, ValidacionError, Validar, limitar, peso_positivo};

/// Ventana máxima del listado de pesajes. Un mes es lo más largo que pide la
/// liquidación del destajo; más que eso es un reporte, no una pantalla, y sin
/// tope un rango de un año obliga a Postgres a recorrer y sumar todo para los
/// totales.
const DIAS_MAXIMOS_PESAJES: i64 = 31;

#[derive(Debug, FromRow, Serialize)]
pub struct MovimientosMateriaPrima {
    pub id: Uuid,
    pub clave_idempotencia: Uuid,

    pub lote_id: Uuid,
    /// Solo el consumo la exige; merma, devolución y ajustes van sin ella.
    pub programacion_id: Option<Uuid>,

    pub tipo: String,
    // NUMERIC(12,2) en la tabla, f64 acá igual que en ingresos: la consulta
    // tiene que castear (`peso::float8 AS "peso!"`) o falla al decodificar.
    pub peso: f64,
    pub peso_efecto: f64,

    pub operario_id: Option<Uuid>,
    pub registrado_por: Uuid,

    pub motivo: Option<String>,
    pub observaciones: Option<String>,

    pub corrige_movimiento_id: Option<Uuid>,

    // Origen de báscula: los siete van todos o ninguno (CHECK de la tabla),
    // y solo en consumos que llegaron desde un concentrador.
    /// Hora real del pesaje, del reloj del ESP32. En los pesajes de báscula
    /// es esta y no `creado_en`, que es cuándo llegó al servidor.
    pub medido_en: Option<DateTime<Utc>>,
    pub recibido_concentrador_en: Option<DateTime<Utc>>,
    pub concentrador_id: Option<Uuid>,
    pub bascula_id: Option<Uuid>,
    pub sesion_arranque: Option<i64>,
    pub transaccion: Option<i64>,
    pub uid_tarjeta: Option<String>,

    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}

/// Lo que devuelve el alta: el movimiento, y si de verdad se creó en esta
/// llamada. Mismo patrón que `AltaIngreso`: `creado: false` es un reintento
/// de la Pi, y el endpoint le responde 409, que ella toma como éxito.
#[derive(Debug)]
pub struct AltaMovimiento {
    pub movimiento: MovimientosMateriaPrima,
    pub creado: bool,
}

/// Un consumo que llegó desde una báscula, ya resuelto: el lote y el operario
/// los decidió el servidor con `medido_en`, no los manda la Pi.
///
/// `tipo` no está porque siempre es 'consumo', y `registrado_por` tampoco:
/// va aparte en el servicio, igual que en el resto de las altas.
#[derive(Debug)]
pub struct ConsumoBasculaNuevo {
    /// El `id` que generó la Pi.
    pub clave_idempotencia: Uuid,

    pub lote_id: Uuid,
    pub programacion_id: Uuid,
    pub operario_id: Uuid,

    /// El f64 de la Pi tal cual: la columna NUMERIC(12,2) lo redondea al
    /// guardar.
    pub peso: f64,

    pub medido_en: DateTime<Utc>,
    pub recibido_concentrador_en: DateTime<Utc>,
    pub concentrador_id: Uuid,
    pub bascula_id: Uuid,
    pub sesion_arranque: i64,
    pub transaccion: i64,
    pub uid_tarjeta: String,
}

/// Cuerpo del POST que manda el concentrador: los campos exactos de
/// `PesajeNube` en la Pi. Si allá cambia el contrato, cambia acá.
///
/// El concentrador no viaja acá: sale de la llave con la que se autenticó la
/// petición, nunca del cuerpo.
#[derive(Debug, Deserialize)]
pub struct PesajeBasculaPayload {
    /// La genera la Pi por pesaje; es la `clave_idempotencia` del movimiento.
    pub id: Uuid,
    pub device_id: String,
    pub sesion_arranque: i64,
    pub transaccion: i64,
    pub uid_tarjeta: String,
    pub peso_kg: f64,
    pub hora_medicion: Option<DateTime<Utc>>,
    pub hora_dispositivo: Option<DateTime<Utc>>,
    pub recibido_en: DateTime<Utc>,
}

/// Lo que devuelve validar(): el pesaje limpio, todavía sin resolver. Con
/// esto el servicio busca la báscula, el lote y el operario, y arma el
/// `ConsumoBasculaNuevo`.
///
/// `hora_dispositivo` no pasa: no tiene columna, y la que decide es
/// `hora_medicion`.
#[derive(Debug)]
pub struct PesajeBascula {
    pub clave_idempotencia: Uuid,
    pub device_id: String,
    pub sesion_arranque: i64,
    pub transaccion: i64,
    pub uid_tarjeta: String,
    pub peso_kg: f64,
    pub medido_en: DateTime<Utc>,
    pub recibido_concentrador_en: DateTime<Utc>,
}

impl Validar for PesajeBasculaPayload {
    type Datos = PesajeBascula;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // Mismo formato que `llaves_nfc.uid`: sin esto una tarjeta leída como
        // "04:a1:b2:c3" no encontraría su llave guardada como "04A1B2C3".
        let uid_tarjeta = uid_nfc(&self.uid_tarjeta)?;

        let peso_kg = peso_positivo(self.peso_kg, "peso_kg")?;

        // Sin hora de medición no se puede decidir ni el lote ni el operario.
        // Es 400 y no 422: el pesaje está mal formado, no es que no se haya
        // podido asignar, y la Pi lo aparta en vez de reintentarlo.
        let medido_en = self
            .hora_medicion
            .ok_or_else(|| ValidacionError::nuevo("el pesaje no trae hora_medicion"))?;

        Ok(PesajeBascula {
            clave_idempotencia: self.id,
            device_id: self.device_id,
            sesion_arranque: self.sesion_arranque,
            transaccion: self.transaccion,
            uid_tarjeta,
            peso_kg,
            medido_en,
            recibido_concentrador_en: self.recibido_en,
        })
    }
}

/// Posición en el listado de pesajes: el (medido_en, id) de la última fila
/// que recibió el cliente. El `id` desempata dos pesajes del mismo instante,
/// que con varias básculas en la línea pasa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct CursorPesaje {
    pub medido_en: DateTime<Utc>,
    pub id: Uuid,
}

/// Lo que manda el cliente.
#[derive(Debug, Deserialize)]
pub struct PesajesReadPayload {
    /// Obligatorio con las dos puntas. Son instantes UTC que el front ya
    /// calculó para el día local de Colombia: el servidor no hace `::date`,
    /// porque eso cortaría el día en la medianoche UTC (las 7 p. m. de acá).
    pub medido_en: Option<Rango<DateTime<Utc>>>,
    pub operario_id: Option<Uuid>,
    pub llave_id: Option<Uuid>,
    pub lote_id: Option<Uuid>,
    pub programacion_id: Option<Uuid>,
    pub bascula_id: Option<Uuid>,
    pub uid: Option<String>,
    pub corregido: Option<bool>,
    pub limite: Option<i64>,
    pub cursor: Option<CursorPesaje>,
}

/// Lo que devuelve validar() y consume el servicio.
///
/// El rango va como dos campos sueltos y no como `RangoValidado`: acá las dos
/// puntas son obligatorias, y así el servicio no tiene que desenvolver
/// Options que validar() ya garantizó.
#[derive(Debug)]
pub struct PesajesFiltros {
    /// Inclusivo.
    pub desde: DateTime<Utc>,
    /// Exclusivo: `medido_en < hasta`, para que el día siguiente arranque
    /// justo donde termina este sin contar dos veces el pesaje de la frontera.
    pub hasta: DateTime<Utc>,
    pub operario_id: Option<Uuid>,
    pub llave_id: Option<Uuid>,
    pub lote_id: Option<Uuid>,
    pub programacion_id: Option<Uuid>,
    pub bascula_id: Option<Uuid>,
    pub uid: Option<String>,
    pub corregido: Option<bool>,
    pub limite: i64,
    pub cursor: Option<CursorPesaje>,
}

/// Una fila del listado de pesajes: el movimiento más lo que hace falta para
/// leerlo sin ir a buscar a otras pantallas (quién, con qué llave, en qué
/// báscula, de qué lote).
#[derive(Debug, FromRow, Serialize)]
pub struct PesajeListado {
    pub id: Uuid,
    pub lote_id: Uuid,
    pub programacion_id: Uuid,
    pub operario_id: Uuid,
    pub bascula_id: Uuid,
    pub peso: f64,
    pub medido_en: DateTime<Utc>,
    pub recibido_concentrador_en: DateTime<Utc>,
    pub creado_en: DateTime<Utc>,
    pub sesion_arranque: i64,
    pub transaccion: i64,
    pub uid_tarjeta: String,

    pub operario_codigo: String,
    pub operario_nombre: String,
    pub operario_apellido: String,

    /// La llave registrada con ese uid. `None` si la tarjeta ya no está en
    /// `llaves_nfc` o se leyó con otro formato: no hay FK entre las dos.
    pub llave_codigo: Option<String>,

    pub bascula_codigo: String,
    pub lote_codigo: String,
    pub materia_prima_nombre: String,

    /// Si algún ajuste apunta a este pesaje con `corrige_movimiento_id`.
    pub corregido: bool,
}

/// Una página del listado. `siguiente_cursor` en `None` es la última página.
#[derive(Debug, Serialize)]
pub struct PaginaPesajes {
    pub pesajes: Vec<PesajeListado>,
    pub siguiente_cursor: Option<CursorPesaje>,
}

/// Totales por operario sobre todo el filtro, no sobre una página.
#[derive(Debug, FromRow, Serialize)]
pub struct TotalPesajesOperario {
    pub operario_id: Uuid,
    pub operario_codigo: String,
    pub operario_nombre: String,
    pub operario_apellido: String,
    pub cantidad: i64,
    /// Brutos: no restan los ajustes que corrigen estos pesajes.
    pub kg: f64,
    pub cantidad_corregidos: i64,
}

impl Validar for PesajesReadPayload {
    type Datos = PesajesFiltros;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        // Sin rango el listado recorrería la tabla entera, y los totales del
        // destajo sin fechas no significan nada: por eso no hay default.
        let rango = self
            .medido_en
            .ok_or_else(|| ValidacionError::nuevo("falta el rango de medido_en"))?
            .validar()?;

        let (Some(desde), Some(hasta)) = (rango.desde(), rango.hasta()) else {
            return Err(ValidacionError::nuevo(
                "el rango de medido_en necesita desde y hasta",
            ));
        };

        if hasta - desde > Duration::days(DIAS_MAXIMOS_PESAJES) {
            return Err(ValidacionError::nuevo(format!(
                "el rango de medido_en no puede pasar de {DIAS_MAXIMOS_PESAJES} días"
            )));
        }

        // Igualdad exacta contra `uid_tarjeta`, que se guardó normalizado con
        // `uid_nfc`: sin normalizar acá, "04:a1:b2:c3" no encontraría nada.
        let uid = match self.uid.as_deref().map(str::trim) {
            None | Some("") => None,
            Some(valor) => Some(uid_nfc(valor)?),
        };

        let limite = limitar(self.limite);

        Ok(PesajesFiltros {
            desde,
            hasta,
            operario_id: self.operario_id,
            llave_id: self.llave_id,
            lote_id: self.lote_id,
            programacion_id: self.programacion_id,
            bascula_id: self.bascula_id,
            uid,
            corregido: self.corregido,
            limite,
            cursor: self.cursor,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::validations::LIMITE_POR_DEFECTO;

    fn payload() -> PesajeBasculaPayload {
        let hora: DateTime<Utc> = "2026-10-20T14:03:11Z".parse().unwrap();
        PesajeBasculaPayload {
            id: Uuid::new_v4(),
            device_id: "70:4b:ca:99:3f:30".into(),
            sesion_arranque: 53,
            transaccion: 128,
            uid_tarjeta: "04:a1:b2:c3".into(),
            peso_kg: 22.4,
            hora_medicion: Some(hora),
            hora_dispositivo: Some(hora),
            recibido_en: hora,
        }
    }

    #[test]
    fn validar_normaliza_y_devuelve_los_datos() {
        let entrada = payload();
        let id = entrada.id;
        let hora = entrada.hora_medicion.unwrap();

        let datos = entrada.validar().unwrap();

        assert_eq!(datos.clave_idempotencia, id);
        assert_eq!(datos.uid_tarjeta, "04A1B2C3");
        assert_eq!(datos.peso_kg, 22.4);
        assert_eq!(datos.medido_en, hora);
        assert_eq!(datos.recibido_concentrador_en, hora);
    }

    #[test]
    fn validar_rechaza_el_pesaje_sin_hora_de_medicion() {
        let entrada = PesajeBasculaPayload {
            hora_medicion: None,
            ..payload()
        };

        assert!(entrada.validar().is_err());
    }

    #[test]
    fn validar_rechaza_pesos_no_positivos() {
        for peso_kg in [0.0, -1.0, f64::NAN] {
            let entrada = PesajeBasculaPayload {
                peso_kg,
                ..payload()
            };
            assert!(entrada.validar().is_err(), "{peso_kg} debió fallar");
        }
    }

    #[test]
    fn validar_rechaza_uid_invalido() {
        let entrada = PesajeBasculaPayload {
            uid_tarjeta: "ZZZZ".into(),
            ..payload()
        };

        assert!(entrada.validar().is_err());
    }

    fn hora(texto: &str) -> DateTime<Utc> {
        texto.parse().unwrap()
    }

    /// Un día local de Colombia (UTC-5) expresado en UTC, como lo manda el
    /// front.
    fn read_payload() -> PesajesReadPayload {
        PesajesReadPayload {
            medido_en: Some(Rango {
                desde: Some(hora("2026-10-20T05:00:00Z")),
                hasta: Some(hora("2026-10-21T05:00:00Z")),
            }),
            operario_id: None,
            llave_id: None,
            lote_id: None,
            programacion_id: None,
            bascula_id: None,
            uid: None,
            corregido: None,
            limite: None,
            cursor: None,
        }
    }

    #[test]
    fn validar_pesajes_devuelve_el_rango_y_el_limite_por_defecto() {
        let filtros = read_payload().validar().expect("deberia ser valido");

        assert_eq!(filtros.desde, hora("2026-10-20T05:00:00Z"));
        assert_eq!(filtros.hasta, hora("2026-10-21T05:00:00Z"));
        assert_eq!(filtros.limite, LIMITE_POR_DEFECTO);
        assert_eq!(filtros.uid, None);
    }

    #[test]
    fn validar_pesajes_rechaza_el_rango_faltante() {
        let sin_rango = PesajesReadPayload {
            medido_en: None,
            ..read_payload()
        };
        assert!(sin_rango.validar().is_err());

        let sin_hasta = PesajesReadPayload {
            medido_en: Some(Rango {
                desde: Some(hora("2026-10-20T05:00:00Z")),
                hasta: None,
            }),
            ..read_payload()
        };
        assert!(sin_hasta.validar().is_err());

        let sin_desde = PesajesReadPayload {
            medido_en: Some(Rango {
                desde: None,
                hasta: Some(hora("2026-10-21T05:00:00Z")),
            }),
            ..read_payload()
        };
        assert!(sin_desde.validar().is_err());
    }

    #[test]
    fn validar_pesajes_rechaza_desde_posterior_a_hasta() {
        let payload = PesajesReadPayload {
            medido_en: Some(Rango {
                desde: Some(hora("2026-10-21T05:00:00Z")),
                hasta: Some(hora("2026-10-20T05:00:00Z")),
            }),
            ..read_payload()
        };

        assert!(payload.validar().is_err());
    }

    /// 31 días justos pasan; un segundo más ya no.
    #[test]
    fn validar_pesajes_limita_la_ventana_a_31_dias() {
        let justo = PesajesReadPayload {
            medido_en: Some(Rango {
                desde: Some(hora("2026-10-01T05:00:00Z")),
                hasta: Some(hora("2026-11-01T05:00:00Z")),
            }),
            ..read_payload()
        };
        assert!(justo.validar().is_ok());

        let pasado = PesajesReadPayload {
            medido_en: Some(Rango {
                desde: Some(hora("2026-10-01T05:00:00Z")),
                hasta: Some(hora("2026-11-01T05:00:01Z")),
            }),
            ..read_payload()
        };
        assert!(pasado.validar().is_err());
    }

    #[test]
    fn validar_pesajes_normaliza_el_uid() {
        let payload = PesajesReadPayload {
            uid: Some(" 04:a1:b2:c3 ".into()),
            ..read_payload()
        };

        let filtros = payload.validar().expect("deberia ser valido");

        assert_eq!(filtros.uid.as_deref(), Some("04A1B2C3"));
    }

    /// Un uid en blanco es "sin filtro", no un error.
    #[test]
    fn validar_pesajes_uid_en_blanco_queda_en_none() {
        let payload = PesajesReadPayload {
            uid: Some("   ".into()),
            ..read_payload()
        };

        let filtros = payload.validar().expect("deberia ser valido");

        assert_eq!(filtros.uid, None);
    }

    #[test]
    fn validar_pesajes_rechaza_uid_invalido() {
        let payload = PesajesReadPayload {
            uid: Some("ZZZZ".into()),
            ..read_payload()
        };

        assert!(payload.validar().is_err());
    }
}
