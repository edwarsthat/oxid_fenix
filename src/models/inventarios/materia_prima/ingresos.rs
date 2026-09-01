use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

use crate::models::validations::{
    ValidacionError, Validar, peso, peso_positivo, texto_opcional, uuid_requerido,
};

#[derive(Debug, FromRow, Serialize)]
pub struct Ingreso {
    pub id: Uuid,
    pub codigo: String,
    pub predio_id: Uuid,
    pub materia_prima_id: Uuid,
    pub placa: String,
    pub numero_remision: Option<String>,
    pub numero_tiquete_bascula: Option<String>,

    // DATE en la tabla, no TIMESTAMPTZ: es el día para agrupar reportes.
    // La hora exacta de llegada va aparte, en `llegada_en`.
    pub fecha_ingreso: NaiveDate,
    pub llegada_en: DateTime<Utc>,
    pub inicio_descargue_en: Option<DateTime<Utc>>,
    pub fin_descargue_en: Option<DateTime<Utc>>,

    // Los pesos son NUMERIC(12,2) en la tabla y viajan como f64, igual que
    // los porcentajes de materias_primas: el driver no trae tipo decimal
    // habilitado, así que el query tiene que castear
    // (`peso_ingreso::float8 AS "peso_ingreso!"`) o falla al decodificar.
    //
    // El f64 es para mostrar. Cualquier cuenta que termine en plata se hace
    // en SQL, donde el valor sigue siendo NUMERIC.
    pub peso_ingreso: f64,
    pub peso_devuelto: f64,

    // NULL = el lote todavía tiene saldo en patio. La pone el trigger del
    // libro de movimientos, no la aplicación.
    pub cerrado_en: Option<DateTime<Utc>>,

    pub observaciones: Option<String>,
    pub registrado_por: Uuid,

    pub anulado_en: Option<DateTime<Utc>>,
    pub anulado_por: Option<Uuid>,
    pub motivo_anulacion: Option<String>,

    pub version: i32,
    pub creado_en: DateTime<Utc>,
    pub actualizado_en: DateTime<Utc>,
}

/// Lo que manda el cliente.
///
/// Los ids y las fechas llegan como texto igual que en `predios`: si se
/// declararan como `Uuid` o `NaiveDate`, un valor mal escrito lo rechazaría
/// serde con un "missing field" o un "invalid value" en inglés, antes de que
/// `validar()` alcance a decir cuál campo es y qué se esperaba.
///
/// `fecha_ingreso` y `llegada_en` son opcionales porque la tabla tiene
/// DEFAULT para las dos: si el de báscula registra el camión en el momento,
/// no tiene por qué mandarlas. Van separadas y no se deriva una de la otra
/// porque `llegada_en` es un instante en UTC y `fecha_ingreso` es el día del
/// calendario local: a las 8pm de Colombia el UTC ya está en el día
/// siguiente, así que derivar el día del instante daría la fecha equivocada
/// justo en el turno de la noche.
///
/// `registrado_por` NO viaja acá: sale de la sesión (`ctx.user_id`), no de
/// lo que diga el cliente.
#[derive(Debug, Deserialize)]
pub struct IngresoAddPayload {
    pub predio_id: Option<String>,
    pub materia_prima_id: Option<String>,
    pub placa: String,
    pub numero_remision: Option<String>,
    pub numero_tiquete_bascula: Option<String>,
    pub fecha_ingreso: Option<String>,
    pub llegada_en: Option<String>,
    pub inicio_descargue_en: Option<String>,
    pub fin_descargue_en: Option<String>,
    pub peso_ingreso: f64,
    pub peso_devuelto: Option<f64>,
    pub observaciones: Option<String>,
}

/// Lo que devuelve validar() y consume el servicio: si tenés un
/// `IngresoNuevo`, los datos ya están normalizados y son válidos.
///
/// Las fechas siguen en `Option` a propósito: `None` significa "que la ponga
/// la base", y el INSERT las mete con COALESCE contra el DEFAULT. No se
/// resuelven acá con `Utc::now()` para que la hora la ponga el reloj del
/// servidor de base y no el del proceso.
#[derive(Debug)]
pub struct IngresoNuevo {
    pub predio_id: Uuid,
    pub materia_prima_id: Uuid,
    pub placa: String,
    pub numero_remision: Option<String>,
    pub numero_tiquete_bascula: Option<String>,
    pub fecha_ingreso: Option<NaiveDate>,
    pub llegada_en: Option<DateTime<Utc>>,
    pub inicio_descargue_en: Option<DateTime<Utc>>,
    pub fin_descargue_en: Option<DateTime<Utc>>,
    pub peso_ingreso: f64,
    pub peso_devuelto: f64,
    pub observaciones: Option<String>,
}

/// La placa se guarda normalizada, como dice el comentario de la migración:
/// mayúsculas y sin espacios ni guiones, para que "abc-123", "ABC 123" y
/// "ABC123" sean la misma placa al buscar. El CHECK de la tabla la atraparía
/// igual, pero como un 23514 genérico y sin haberla normalizado.
fn placa(valor: &str) -> Result<String, ValidacionError> {
    let placa: String = valor
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_uppercase();

    // El mismo rango que `ingresos_materia_prima_placa_check`: cubre camión
    // (ABC123), remolque (R12345) y moto (ABC12D) sin casarse con un formato.
    if !(5..=7).contains(&placa.chars().count()) {
        return Err(ValidacionError::nuevo(
            "la placa debe tener entre 5 y 7 caracteres alfanuméricos",
        ));
    }

    Ok(placa)
}

/// Día del calendario, como lo manda un `<input type="date">`.
fn fecha_opcional(valor: Option<&str>, campo: &str) -> Result<Option<NaiveDate>, ValidacionError> {
    match valor.map(str::trim) {
        None | Some("") => Ok(None),
        Some(texto) => NaiveDate::parse_from_str(texto, "%Y-%m-%d")
            .map(Some)
            .map_err(|_| ValidacionError::nuevo(format!("la {campo} debe venir como AAAA-MM-DD"))),
    }
}

/// Instante con zona horaria. Se exige la zona (RFC 3339) y no se asume UTC:
/// un "2026-08-31T20:00:00" a secas son dos horas distintas según quién lo
/// interprete, y acá la diferencia decide si un lote de yuca ya se venció.
fn momento_opcional(
    valor: Option<&str>,
    campo: &str,
) -> Result<Option<DateTime<Utc>>, ValidacionError> {
    match valor.map(str::trim) {
        None | Some("") => Ok(None),
        Some(texto) => DateTime::parse_from_rfc3339(texto)
            .map(|momento| Some(momento.with_timezone(&Utc)))
            .map_err(|_| {
                ValidacionError::nuevo(format!(
                    "{campo} debe venir en formato ISO 8601 con zona horaria (2026-08-31T05:30:00-05:00)"
                ))
            }),
    }
}

impl Validar for IngresoAddPayload {
    type Datos = IngresoNuevo;

    fn validar(self) -> Result<Self::Datos, ValidacionError> {
        let predio_id = uuid_requerido(self.predio_id, "predio_id")?;
        let materia_prima_id = uuid_requerido(self.materia_prima_id, "materia_prima_id")?;

        let placa = placa(&self.placa)?;

        // Los largos son los de la migración: se cortan acá para que Postgres
        // no devuelva un 22001 (value too long) como error 500.
        let numero_remision =
            texto_opcional(self.numero_remision.as_deref(), "numero_remision", 50)?;
        let numero_tiquete_bascula = texto_opcional(
            self.numero_tiquete_bascula.as_deref(),
            "numero_tiquete_bascula",
            50,
        )?;
        let observaciones = texto_opcional(self.observaciones.as_deref(), "observaciones", 500)?;

        // `peso_positivo` es el que corresponde al CHECK
        // `ingresos_materia_prima_peso_ingreso_check`.
        let peso_ingreso = peso_positivo(self.peso_ingreso, "peso_ingreso")?;

        // Ausente es cero, no NULL: la columna es NOT NULL DEFAULT 0.
        let peso_devuelto = peso(self.peso_devuelto.unwrap_or(0.0), "peso_devuelto")?;

        let fecha_ingreso = fecha_opcional(self.fecha_ingreso.as_deref(), "fecha_ingreso")?;
        let llegada_en = momento_opcional(self.llegada_en.as_deref(), "llegada_en")?;
        let inicio_descargue_en =
            momento_opcional(self.inicio_descargue_en.as_deref(), "inicio_descargue_en")?;
        let fin_descargue_en =
            momento_opcional(self.fin_descargue_en.as_deref(), "fin_descargue_en")?;

        // Las tres reglas siguientes son las mismas de los CHECK
        // `..._inicio_descargue_check` y `..._fin_descargue_check`. Se repiten
        // acá para que el mensaje diga qué corregir en vez de un 23514.
        //
        // El primer par solo se puede comparar si el cliente mandó la llegada:
        // si la va a poner el DEFAULT de la tabla, el CHECK la valida allá.
        if let (Some(inicio), Some(llegada)) = (inicio_descargue_en, llegada_en)
            && inicio < llegada
        {
            return Err(ValidacionError::nuevo(
                "el descargue no puede empezar antes de que llegue el camión",
            ));
        }

        if fin_descargue_en.is_some() && inicio_descargue_en.is_none() {
            return Err(ValidacionError::nuevo(
                "no puede haber fin de descargue sin hora de inicio",
            ));
        }

        if let (Some(inicio), Some(fin)) = (inicio_descargue_en, fin_descargue_en)
            && fin < inicio
        {
            return Err(ValidacionError::nuevo(
                "el descargue no puede terminar antes de empezar",
            ));
        }

        Ok(IngresoNuevo {
            predio_id,
            materia_prima_id,
            placa,
            numero_remision,
            numero_tiquete_bascula,
            fecha_ingreso,
            llegada_en,
            inicio_descargue_en,
            fin_descargue_en,
            peso_ingreso,
            peso_devuelto,
            observaciones,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::validations::PESO_MAXIMO;

    fn add_payload() -> IngresoAddPayload {
        IngresoAddPayload {
            predio_id: Some(Uuid::new_v4().to_string()),
            materia_prima_id: Some(Uuid::new_v4().to_string()),
            placa: "ABC123".into(),
            numero_remision: None,
            numero_tiquete_bascula: None,
            fecha_ingreso: None,
            llegada_en: None,
            inicio_descargue_en: None,
            fin_descargue_en: None,
            peso_ingreso: 3000.0,
            peso_devuelto: None,
            observaciones: None,
        }
    }

    #[test]
    fn validar_normaliza_y_devuelve_los_datos() {
        let predio_id = Uuid::new_v4();
        let materia_prima_id = Uuid::new_v4();
        let payload = IngresoAddPayload {
            predio_id: Some(format!("  {predio_id}  ")),
            materia_prima_id: Some(materia_prima_id.to_string()),
            placa: " abc-123 ".into(),
            numero_remision: Some("  R-4471  ".into()),
            fecha_ingreso: Some("2026-08-31".into()),
            llegada_en: Some("2026-08-31T05:30:00-05:00".into()),
            ..add_payload()
        };

        let nuevo = payload.validar().expect("deberia ser valido");

        assert_eq!(nuevo.predio_id, predio_id);
        assert_eq!(nuevo.materia_prima_id, materia_prima_id);
        assert_eq!(nuevo.placa, "ABC123");
        assert_eq!(nuevo.numero_remision.as_deref(), Some("R-4471"));
        assert_eq!(
            nuevo.fecha_ingreso,
            Some(NaiveDate::from_ymd_opt(2026, 8, 31).unwrap())
        );
        // 05:30 en Colombia son las 10:30 UTC: la zona se aplica, no se ignora.
        assert_eq!(
            nuevo.llegada_en.map(|t| t.to_rfc3339()),
            Some("2026-08-31T10:30:00+00:00".to_string())
        );
    }

    /// Las fechas ausentes quedan en None para que las ponga el DEFAULT de la
    /// tabla, no el reloj de este proceso.
    #[test]
    fn las_fechas_ausentes_o_en_blanco_quedan_en_none() {
        let payload = IngresoAddPayload {
            fecha_ingreso: Some("   ".into()),
            llegada_en: None,
            ..add_payload()
        };

        let nuevo = payload.validar().expect("deberia ser valido");

        assert_eq!(nuevo.fecha_ingreso, None);
        assert_eq!(nuevo.llegada_en, None);
    }

    /// La columna es NOT NULL DEFAULT 0: ausente es cero, no NULL.
    #[test]
    fn el_peso_devuelto_ausente_es_cero() {
        let nuevo = add_payload().validar().expect("deberia ser valido");

        assert_eq!(nuevo.peso_devuelto, 0.0);
    }

    #[test]
    fn los_opcionales_en_blanco_quedan_en_none() {
        let payload = IngresoAddPayload {
            numero_remision: Some("   ".into()),
            numero_tiquete_bascula: Some("".into()),
            observaciones: Some("  ".into()),
            ..add_payload()
        };

        let nuevo = payload.validar().expect("deberia ser valido");

        assert_eq!(nuevo.numero_remision, None);
        assert_eq!(nuevo.numero_tiquete_bascula, None);
        assert_eq!(nuevo.observaciones, None);
    }

    #[test]
    fn validar_rechaza_los_ids_ausentes_o_invalidos() {
        let payload = IngresoAddPayload {
            predio_id: None,
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "falta el predio_id"
        );

        let payload = IngresoAddPayload {
            materia_prima_id: Some("no-es-un-uuid".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el materia_prima_id no es un UUID válido"
        );
    }

    /// Mismo rango que `ingresos_materia_prima_placa_check`, con mensaje.
    #[test]
    fn validar_rechaza_placas_fuera_de_rango() {
        for placa in ["AB1", "ABCD1234", "  ", "--"] {
            let payload = IngresoAddPayload {
                placa: placa.into(),
                ..add_payload()
            };

            assert_eq!(
                payload.validar().unwrap_err().mensaje(),
                "la placa debe tener entre 5 y 7 caracteres alfanuméricos",
                "la placa {placa:?} debería rechazarse"
            );
        }
    }

    #[test]
    fn validar_rechaza_pesos_invalidos() {
        let payload = IngresoAddPayload {
            peso_ingreso: 0.0,
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el peso_ingreso debe ser mayor que cero"
        );

        // 0.004 se redondearía a 0.00 al guardarse y reventaría el CHECK.
        let payload = IngresoAddPayload {
            peso_ingreso: 0.004,
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el peso_ingreso debe ser mayor que cero"
        );

        let payload = IngresoAddPayload {
            peso_devuelto: Some(-1.0),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el peso_devuelto no puede ser negativo"
        );
    }

    /// Un peso que no cabe en NUMERIC(12,2) es un 400, no un 500 del driver.
    #[test]
    fn validar_rechaza_un_peso_que_desborda_la_columna() {
        let payload = IngresoAddPayload {
            peso_ingreso: PESO_MAXIMO + 1.0,
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el peso_ingreso supera el máximo que admite la columna"
        );
    }

    #[test]
    fn validar_rechaza_fechas_mal_formadas() {
        let payload = IngresoAddPayload {
            fecha_ingreso: Some("31/08/2026".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "la fecha_ingreso debe venir como AAAA-MM-DD"
        );
    }

    /// Sin zona horaria no se sabe de qué instante se habla, y de eso depende
    /// si un lote de yuca ya pasó sus horas máximas de espera.
    #[test]
    fn validar_rechaza_un_momento_sin_zona_horaria() {
        let payload = IngresoAddPayload {
            llegada_en: Some("2026-08-31T05:30:00".into()),
            ..add_payload()
        };

        assert!(
            payload
                .validar()
                .unwrap_err()
                .mensaje()
                .starts_with("llegada_en debe venir en formato ISO 8601")
        );
    }

    /// Las mismas reglas que los CHECK de descargue de la tabla.
    #[test]
    fn validar_rechaza_tiempos_de_descargue_incoherentes() {
        let payload = IngresoAddPayload {
            llegada_en: Some("2026-08-31T05:30:00-05:00".into()),
            inicio_descargue_en: Some("2026-08-31T05:00:00-05:00".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el descargue no puede empezar antes de que llegue el camión"
        );

        let payload = IngresoAddPayload {
            fin_descargue_en: Some("2026-08-31T07:00:00-05:00".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "no puede haber fin de descargue sin hora de inicio"
        );

        let payload = IngresoAddPayload {
            inicio_descargue_en: Some("2026-08-31T07:00:00-05:00".into()),
            fin_descargue_en: Some("2026-08-31T06:00:00-05:00".into()),
            ..add_payload()
        };

        assert_eq!(
            payload.validar().unwrap_err().mensaje(),
            "el descargue no puede terminar antes de empezar"
        );
    }

    /// Sin `llegada_en` no hay contra qué comparar el inicio: esa regla la
    /// verifica el CHECK contra el DEFAULT, no nosotros.
    #[test]
    fn el_inicio_de_descargue_pasa_si_no_vino_la_llegada() {
        let payload = IngresoAddPayload {
            llegada_en: None,
            inicio_descargue_en: Some("2026-08-31T06:00:00-05:00".into()),
            fin_descargue_en: Some("2026-08-31T07:00:00-05:00".into()),
            ..add_payload()
        };

        assert!(payload.validar().is_ok());
    }
}
