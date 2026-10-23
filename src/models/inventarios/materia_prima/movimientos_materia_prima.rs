use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::inventarios::llaves_nfc::uid_nfc;
use crate::models::validations::{ValidacionError, Validar, peso_positivo};

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
        let medido_en = self.hora_medicion.ok_or_else(|| {
            ValidacionError::nuevo("el pesaje no trae hora_medicion")
        })?;

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

#[cfg(test)]
mod tests {
    use super::*;

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
            let entrada = PesajeBasculaPayload { peso_kg, ..payload() };
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
}
